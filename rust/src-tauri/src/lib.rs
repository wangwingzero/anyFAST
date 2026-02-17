//! anyrouter FAST - Rust Backend
//! 中转站端点优选工具

mod config;
mod endpoint_tester;
mod health_checker;
mod history;
mod hosts_manager;
mod hosts_ops;
mod models;

// Service module (Windows only)
#[cfg(windows)]
pub mod service;

// Client module for communicating with the service
pub mod client;

use config::ConfigManager;
use endpoint_tester::{estimate_test_timeout, EndpointTester};
use health_checker::BaselineTracker;
use history::HistoryManager;
use hosts_manager::HostsBinding;
use models::{
    AppConfig, Endpoint, EndpointResult, HistoryRecord, HistoryStats, PermissionStatus, UpdateInfo,
};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WindowEvent,
};
use tokio::sync::Mutex;

pub struct AppState {
    config_manager: ConfigManager,
    history_manager: HistoryManager,
    tester: Arc<Mutex<Option<EndpointTester>>>,
    results: Arc<Mutex<Vec<EndpointResult>>>,
    baselines: BaselineTracker,
}

/// 从端点 URL 中提取目标域名
/// URL 格式: https://betterclau.de/claude/{target_domain}
/// 返回目标域名，如果无法解析则返回端点的 name 字段
fn extract_target_domain(endpoint: &Endpoint) -> String {
    // 尝试从 URL 路径中提取最后一段作为目标域名
    // 移除末尾的斜杠，然后按 '/' 分割取最后一段
    let url = endpoint.url.trim_end_matches('/');
    if let Some(last_slash) = url.rfind('/') {
        let last_segment = &url[last_slash + 1..];
        // 验证看起来像域名（包含点号且不为空）
        if !last_segment.is_empty() && last_segment.contains('.') {
            return last_segment.to_string();
        }
    }
    // 回退到端点名称
    endpoint.name.clone()
}

/// 按 domain 聚合成功结果，保留延迟最低的 IP
fn collect_best_success_by_domain(results: &[EndpointResult]) -> HashMap<String, (String, f64)> {
    let mut best_by_domain = HashMap::new();

    for result in results.iter().filter(|r| r.success) {
        best_by_domain
            .entry(result.endpoint.domain.clone())
            .and_modify(|(best_ip, best_latency): &mut (String, f64)| {
                if result.latency < *best_latency {
                    *best_ip = result.ip.clone();
                    *best_latency = result.latency;
                }
            })
            .or_insert_with(|| (result.ip.clone(), result.latency));
    }

    best_by_domain
}

/// 仅保留与当前 hosts 不同的绑定，避免无变化写入触发 DNS 刷新
fn filter_changed_bindings(bindings: Vec<HostsBinding>) -> Vec<HostsBinding> {
    bindings
        .into_iter()
        .filter(|binding| {
            hosts_ops::read_binding(&binding.domain).as_deref() != Some(binding.ip.as_str())
        })
        .collect()
}

/// 归一化用户配置的优选 IP 列表：去空、校验、去重并保持原有顺序
fn normalize_preferred_ips(raw_ips: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for ip in raw_ips {
        let trimmed = ip.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(parsed) = trimmed.parse::<IpAddr>() {
            let canonical = parsed.to_string();
            if seen.insert(canonical.clone()) {
                normalized.push(canonical);
            }
        }
    }

    normalized
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    state.config_manager.load().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    let mut config = config;
    config.preferred_ips = normalize_preferred_ips(config.preferred_ips);
    state
        .config_manager
        .save(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_speed_test(
    state: State<'_, AppState>,
    update_baseline: Option<bool>,
) -> Result<Vec<EndpointResult>, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let endpoints: Vec<Endpoint> = config.endpoints.into_iter().filter(|e| e.enabled).collect();

    if endpoints.is_empty() {
        return Err("没有启用的端点".into());
    }

    let update_baseline = update_baseline.unwrap_or(true);

    let tester = EndpointTester::new(config.preferred_ips.clone(), config.test_count);

    // 保存 tester 以便取消
    {
        let mut t = state.tester.lock().await;
        *t = Some(tester.clone());
    }

    // 使用动态全局超时，避免大量端点时后排任务被过早判失败
    let workflow_timeout = estimate_test_timeout(endpoints.len());
    let test_future = tester.test_all(&endpoints);
    let results = match tokio::time::timeout(workflow_timeout, test_future).await {
        Ok(results) => results,
        Err(_) => {
            // 超时，取消测试
            tester.cancel();
            // 清除 tester
            let mut t = state.tester.lock().await;
            *t = None;
            return Err(format!(
                "测速超时（{}秒），请检查网络连接",
                workflow_timeout.as_secs()
            ));
        }
    };

    // 清除 tester
    {
        let mut t = state.tester.lock().await;
        *t = None;
    }

    if update_baseline {
        let baselines = state.baselines.get_baselines_arc();

        let best_by_domain = collect_best_success_by_domain(&results);
        let mut b = baselines.lock().await;
        for (domain, (_, latency)) in best_by_domain {
            b.insert(domain, latency);
        }
    }

    let mut state_results = state.results.lock().await;
    *state_results = results.clone();

    Ok(results)
}

#[tauri::command]
async fn stop_speed_test(state: State<'_, AppState>) -> Result<(), String> {
    let mut tester = state.tester.lock().await;
    if let Some(t) = tester.take() {
        t.cancel();
    }
    Ok(())
}

#[tauri::command]
async fn apply_endpoint(
    state: State<'_, AppState>,
    domain: String,
    ip: String,
    latency: Option<f64>,
) -> Result<(), String> {
    if hosts_ops::read_binding(&domain).as_deref() == Some(ip.as_str()) {
        if let Some(latency) = latency {
            let baselines = state.baselines.get_baselines_arc();
            let mut b = baselines.lock().await;
            b.insert(domain, latency);
        }
        return Ok(());
    }

    hosts_ops::write_binding(&domain, &ip).map_err(|e| e.to_string())?;
    hosts_ops::flush_dns().map_err(|e| e.to_string())?;
    if let Some(latency) = latency {
        let baselines = state.baselines.get_baselines_arc();
        let mut b = baselines.lock().await;
        b.insert(domain.clone(), latency);
    }
    Ok(())
}

#[tauri::command]
async fn apply_all_endpoints(state: State<'_, AppState>) -> Result<u32, String> {
    let results = state.results.lock().await;

    // 获取当前时间戳
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 获取 baselines arc
    let baselines = state.baselines.get_baselines_arc();

    // 收集所有成功的端点绑定（按 domain 去重，取最优结果）
    let best_by_domain = collect_best_success_by_domain(&results);
    let mut bindings: Vec<HostsBinding> = Vec::with_capacity(best_by_domain.len());
    let mut history_records: Vec<HistoryRecord> = Vec::new();
    let mut baseline_updates: Vec<(String, f64)> = Vec::with_capacity(best_by_domain.len());

    for r in results.iter().filter(|r| r.success) {
        // 记录历史
        history_records.push(HistoryRecord {
            timestamp: now,
            domain: extract_target_domain(&r.endpoint),
            original_latency: r.original_latency,
            optimized_latency: r.latency,
            speedup_percent: r.speedup_percent,
            applied: true, // 总是应用
        });
    }

    for (domain, (ip, latency)) in best_by_domain {
        bindings.push(HostsBinding {
            domain: domain.clone(),
            ip,
        });
        baseline_updates.push((domain, latency));
    }
    bindings = filter_changed_bindings(bindings);

    // 批量更新基准延迟
    {
        let mut b = baselines.lock().await;
        for (domain, latency) in baseline_updates {
            b.insert(domain, latency);
        }
    }

    // 保存历史记录
    if let Err(e) = state.history_manager.add_records(history_records) {
        eprintln!("Failed to save history: {}", e);
    }

    if bindings.is_empty() {
        return Ok(0);
    }

    // Apply all bindings in a single file operation
    let count = hosts_ops::write_bindings_batch(&bindings).map_err(|e| e.to_string())?;
    hosts_ops::flush_dns().map_err(|e| e.to_string())?;

    Ok(count as u32)
}

#[tauri::command]
async fn clear_all_bindings(state: State<'_, AppState>) -> Result<u32, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;

    // Collect all domains
    let domains: Vec<&str> = config.endpoints.iter().map(|e| e.domain.as_str()).collect();

    if domains.is_empty() {
        return Ok(0);
    }

    // Clear all bindings in a single file operation
    let count = hosts_ops::clear_bindings_batch(&domains).map_err(|e| e.to_string())?;

    if count > 0 {
        hosts_ops::flush_dns().map_err(|e| e.to_string())?;
    }

    Ok(count as u32)
}

#[tauri::command]
async fn get_bindings(state: State<'_, AppState>) -> Result<Vec<(String, Option<String>)>, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let mut bindings = Vec::new();

    for endpoint in config.endpoints {
        let ip = hosts_ops::read_binding(&endpoint.domain);
        bindings.push((endpoint.domain, ip));
    }

    Ok(bindings)
}

#[tauri::command]
async fn get_binding_count(state: State<'_, AppState>) -> Result<u32, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let mut count = 0;

    for endpoint in config.endpoints {
        if hosts_ops::read_binding(&endpoint.domain).is_some() {
            count += 1;
        }
    }

    Ok(count)
}

#[tauri::command]
fn check_admin() -> bool {
    let (has_permission, _is_using_service) = hosts_ops::get_permission_status();
    has_permission
}

/// Check if the hosts service is running
#[tauri::command]
fn is_service_running() -> bool {
    hosts_ops::is_service_running()
}

/// Get permission status as a structured object
#[tauri::command]
fn get_permission_status() -> PermissionStatus {
    let (has_permission, is_using_service) = hosts_ops::get_permission_status();
    PermissionStatus {
        has_permission,
        is_using_service,
    }
}

/// Refresh service status check
#[tauri::command]
fn refresh_service_status() -> bool {
    hosts_ops::refresh_service_status()
}

/// Check if macOS helper is available
#[tauri::command]
fn is_macos_helper_available() -> bool {
    hosts_ops::is_macos_helper_available()
}

/// Install macOS helper with setuid bit using osascript (shows system password dialog)
/// Returns Ok(true) if installation succeeded, Ok(false) if helper not found in bundle
#[tauri::command]
async fn install_macos_helper() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get the bundled helper path
        let bundled_path = match hosts_ops::get_bundled_helper_path() {
            Some(p) => p,
            None => return Ok(false), // Helper not found in bundle
        };

        let install_path = "/usr/local/bin/anyfast-helper-macos";

        // Build the installation script
        let script = format!(
            r#"
            do shell script "cp '{}' '{}' && chown root:wheel '{}' && chmod 4755 '{}'" with administrator privileges
            "#,
            bundled_path.display(),
            install_path,
            install_path,
            install_path
        );

        // Execute with osascript (will show system password dialog)
        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("无法执行 osascript: {}", e))?;

        if output.status.success() {
            // Refresh the helper status so it gets re-detected without restart
            hosts_ops::refresh_macos_helper_status();
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("User canceled") || stderr.contains("canceled") {
                Err("用户取消了安装".to_string())
            } else {
                Err(format!("安装失败: {}", stderr))
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }
}

/// Check if bundled helper exists (for showing install button)
#[tauri::command]
fn has_bundled_helper() -> bool {
    hosts_ops::get_bundled_helper_path().is_some()
}

#[tauri::command]
fn get_hosts_path() -> String {
    #[cfg(windows)]
    {
        r"C:\Windows\System32\drivers\etc\hosts".to_string()
    }

    #[cfg(not(windows))]
    {
        "/etc/hosts".to_string()
    }
}

#[tauri::command]
async fn open_hosts_file() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        // 使用 notepad 打开 hosts 文件
        Command::new("notepad")
            .arg(r"C:\Windows\System32\drivers\etc\hosts")
            .spawn()
            .map_err(|e| format!("无法打开 hosts 文件: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open")
            .arg("-t")
            .arg("/etc/hosts")
            .spawn()
            .map_err(|e| format!("无法打开 hosts 文件: {}", e))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdg-open")
            .arg("/etc/hosts")
            .spawn()
            .map_err(|e| format!("无法打开 hosts 文件: {}", e))?;
        Ok(())
    }
}

#[tauri::command]
async fn get_history_stats(state: State<'_, AppState>, hours: u32) -> Result<HistoryStats, String> {
    state
        .history_manager
        .get_stats(hours)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history_manager.clear_all().map_err(|e| e.to_string())
}

// ===== 单端点解绑命令 =====

/// 解绑单个端点的 hosts 绑定
#[tauri::command]
async fn unbind_endpoint(domain: String) -> Result<(), String> {
    if hosts_ops::read_binding(&domain).is_none() {
        return Ok(()); // 没有绑定，无需操作
    }
    hosts_ops::clear_binding(&domain).map_err(|e| e.to_string())?;
    hosts_ops::flush_dns().map_err(|e| e.to_string())?;
    Ok(())
}

/// 检查是否有活跃的 hosts 绑定
#[tauri::command]
async fn has_any_bindings(state: State<'_, AppState>) -> Result<bool, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    for endpoint in &config.endpoints {
        if hosts_ops::read_binding(&endpoint.domain).is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

// ===== 单端点测速命令 =====

/// 单独测试一个端点，返回测速结果并更新状态
#[tauri::command]
async fn test_single_endpoint(
    state: State<'_, AppState>,
    endpoint: Endpoint,
) -> Result<EndpointResult, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let tester = EndpointTester::new(config.preferred_ips.clone(), config.test_count);

    // 使用 30 秒超时防止永久卡住
    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tester.test_endpoint(&endpoint),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            return Err("单端点测速超时（30秒），请检查网络连接".into());
        }
    };

    // 更新全局结果列表中该端点的结果
    {
        let mut state_results = state.results.lock().await;
        if let Some(existing) = state_results
            .iter_mut()
            .find(|r| r.endpoint.domain == endpoint.domain)
        {
            *existing = result.clone();
        } else {
            state_results.push(result.clone());
        }
    }

    // 如果测速成功，更新基准延迟
    if result.success {
        let baselines = state.baselines.get_baselines_arc();
        let mut b = baselines.lock().await;
        b.insert(endpoint.domain.clone(), result.latency);
    }

    Ok(result)
}

// ===== 获取当前测速结果 =====

/// 获取当前测速结果
/// 用于程序启动时恢复已有的测速数据
#[tauri::command]
async fn get_current_results(state: State<'_, AppState>) -> Result<Vec<EndpointResult>, String> {
    let results = state.results.lock().await;
    Ok(results.clone())
}

// 当前版本号（从 tauri.conf.json 读取，通过 build.rs 设置）
const CURRENT_VERSION: &str = env!("APP_VERSION");

// GitHub 仓库信息
const GITHUB_REPO: &str = "wangwingzero/anyFAST";

/// 检查更新
#[tauri::command]
async fn check_for_update() -> Result<UpdateInfo, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let client = reqwest::Client::builder()
        .user_agent("anyFAST")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("请求 GitHub API 失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API 返回错误: {}", response.status()));
    }

    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let latest_version = release["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    let release_notes = release["body"].as_str().unwrap_or("").to_string();
    let release_url = release["html_url"]
        .as_str()
        .unwrap_or(&format!(
            "https://github.com/{}/releases/latest",
            GITHUB_REPO
        ))
        .to_string();
    let published_at = release["published_at"].as_str().unwrap_or("").to_string();

    // 比较版本号
    let has_update = compare_versions(&latest_version, CURRENT_VERSION);

    Ok(UpdateInfo {
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        has_update,
        release_url,
        release_notes,
        published_at,
    })
}

/// 比较版本号，返回 true 如果 latest > current
fn compare_versions(latest: &str, current: &str) -> bool {
    let parse_version =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

    let latest_parts = parse_version(latest);
    let current_parts = parse_version(current);

    for i in 0..latest_parts.len().max(current_parts.len()) {
        let l = latest_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }
    false
}

/// 获取当前版本号
#[tauri::command]
fn get_current_version() -> String {
    CURRENT_VERSION.to_string()
}

// ===== 开机自启动命令 =====

// Windows 注册表路径和应用名称
#[cfg(target_os = "windows")]
const AUTOSTART_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const APP_NAME: &str = "anyFAST";

/// 设置开机自启动
/// Requirements: 1.3
#[tauri::command]
async fn set_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(AUTOSTART_KEY)
            .map_err(|e| format!("无法打开注册表: {}", e))?;

        if enabled {
            // 获取当前可执行文件路径
            let exe_path =
                std::env::current_exe().map_err(|e| format!("无法获取程序路径: {}", e))?;
            let exe_str = exe_path.to_string_lossy().to_string();

            // 写入注册表
            key.set_value(APP_NAME, &exe_str)
                .map_err(|e| format!("无法写入注册表: {}", e))?;
        } else {
            // 删除注册表项（忽略不存在的情况）
            let _ = key.delete_value(APP_NAME);
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows 平台暂不支持
        let _ = enabled;
        Err("开机自启动功能仅在 Windows 上可用".to_string())
    }
}

/// 获取开机自启动状态
/// Requirements: 1.3
#[tauri::command]
async fn get_autostart() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);

        // 尝试打开注册表键
        let key = match hkcu.open_subkey(AUTOSTART_KEY) {
            Ok(k) => k,
            Err(_) => return Ok(false), // 键不存在，返回 false
        };

        // 检查是否存在 anyFAST 值
        let result: Result<String, _> = key.get_value(APP_NAME);
        Ok(result.is_ok())
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows 平台返回 false
        Ok(false)
    }
}

/// Restart the application as administrator
#[tauri::command]
async fn restart_as_admin() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let exe_path =
            std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;

        let exe_str: Vec<u16> = OsStr::new(exe_path.as_os_str())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let verb: Vec<u16> = OsStr::new("runas")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR::from_raw(verb.as_ptr()),
                PCWSTR::from_raw(exe_str.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };

        // ShellExecuteW returns > 32 on success
        if result.0 as usize > 32 {
            // Exit current instance
            std::process::exit(0);
        } else {
            Err("用户取消了管理员权限请求".to_string())
        }
    }

    #[cfg(not(windows))]
    {
        Err("此功能仅在 Windows 上可用".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let config_manager = ConfigManager::new();

            let state = AppState {
                config_manager: config_manager.clone(),
                history_manager: HistoryManager::new(),
                tester: Arc::new(Mutex::new(None)),
                results: Arc::new(Mutex::new(Vec::new())),
                baselines: BaselineTracker::new(),
            };
            app.manage(state);

            // 创建托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            // 创建托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.set_skip_taskbar(false);
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            // 退出时保留 hosts 绑定，用户可通过解绑功能手动清除
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键点击托盘图标显示窗口
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.set_skip_taskbar(false);
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // 处理窗口关闭事件 - 始终最小化到托盘
            let app_handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        // 阻止关闭，改为隐藏窗口到托盘
                        api.prevent_close();
                        if let Some(win) = app_handle.get_webview_window("main") {
                            let _ = win.set_skip_taskbar(true);
                            let _ = win.hide();
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            start_speed_test,
            stop_speed_test,
            apply_endpoint,
            apply_all_endpoints,
            clear_all_bindings,
            unbind_endpoint,
            has_any_bindings,
            get_bindings,
            get_binding_count,
            check_admin,
            is_service_running,
            get_permission_status,
            refresh_service_status,
            is_macos_helper_available,
            install_macos_helper,
            has_bundled_helper,
            get_hosts_path,
            open_hosts_file,
            get_history_stats,
            clear_history,
            // 单端点测速
            test_single_endpoint,
            get_current_results,
            // 开机自启动
            set_autostart,
            get_autostart,
            // 权限
            restart_as_admin,
            // 更新检查
            check_for_update,
            get_current_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_preferred_ips_should_trim_dedupe_and_drop_invalid() {
        let input = vec![
            " 104.26.13.202 ".to_string(),
            "104.26.13.202".to_string(),
            "bad-ip".to_string(),
            "".to_string(),
            "172.67.74.246".to_string(),
        ];

        let got = normalize_preferred_ips(input);
        assert_eq!(
            got,
            vec!["104.26.13.202".to_string(), "172.67.74.246".to_string()]
        );
    }

    #[test]
    fn normalize_preferred_ips_should_support_ipv6() {
        let input = vec!["::1".to_string(), " ::1 ".to_string()];
        let got = normalize_preferred_ips(input);
        assert_eq!(got, vec!["::1".to_string()]);
    }
}
