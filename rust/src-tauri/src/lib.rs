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
use endpoint_tester::EndpointTester;
use health_checker::{HealthChecker, HealthStatus};
use history::HistoryManager;
use hosts_manager::HostsBinding;
use models::{
    AppConfig, Endpoint, EndpointResult, HistoryRecord, HistoryStats, PermissionStatus, UpdateInfo,
    WorkflowResult,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State, WindowEvent,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    config_manager: ConfigManager,
    history_manager: HistoryManager,
    tester: Arc<Mutex<Option<EndpointTester>>>,
    results: Arc<Mutex<Vec<EndpointResult>>>,
    // 自动模式相关
    health_checker: Arc<Mutex<HealthChecker>>,
    auto_mode_token: Arc<Mutex<Option<CancellationToken>>>,
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    state.config_manager.load().map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    state
        .config_manager
        .save(&config)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_speed_test(state: State<'_, AppState>) -> Result<Vec<EndpointResult>, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let endpoints: Vec<Endpoint> = config.endpoints.into_iter().filter(|e| e.enabled).collect();

    if endpoints.is_empty() {
        return Err("没有启用的端点".into());
    }

    let tester = EndpointTester::new(vec![]);

    // 保存 tester 以便取消
    {
        let mut t = state.tester.lock().await;
        *t = Some(tester.clone());
    }

    // 使用全局超时（60秒）防止永久卡住
    let test_future = tester.test_all(&endpoints);
    let results = match tokio::time::timeout(std::time::Duration::from_secs(60), test_future).await
    {
        Ok(results) => results,
        Err(_) => {
            // 超时，取消测试
            tester.cancel();
            // 清除 tester
            let mut t = state.tester.lock().await;
            *t = None;
            return Err("测速超时（60秒），请检查网络连接".into());
        }
    };

    // 清除 tester
    {
        let mut t = state.tester.lock().await;
        *t = None;
    }

    // 更新基准延迟（避免长时间持有 health_checker 锁）
    let baselines = {
        let checker = state.health_checker.lock().await;
        checker.get_baselines_arc()
    }; // health_checker 锁在此释放

    for r in &results {
        if r.success {
            let mut b = baselines.lock().await;
            b.insert(r.endpoint.domain.clone(), r.latency);
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
async fn apply_endpoint(domain: String, ip: String) -> Result<(), String> {
    hosts_ops::write_binding(&domain, &ip).map_err(|e| e.to_string())?;
    hosts_ops::flush_dns().map_err(|e| e.to_string())
}

#[tauri::command]
async fn apply_all_endpoints(state: State<'_, AppState>) -> Result<u32, String> {
    let results = state.results.lock().await;

    // 获取当前时间戳
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 获取 baselines arc（避免长时间持有 health_checker 锁）
    let baselines = {
        let checker = state.health_checker.lock().await;
        checker.get_baselines_arc()
    }; // health_checker 锁在此释放

    // 收集所有成功的端点绑定（无论是原始 IP 还是优化 IP，都绑定最优的）
    let mut bindings: Vec<HostsBinding> = Vec::new();
    let mut history_records: Vec<HistoryRecord> = Vec::new();
    let mut baseline_updates: Vec<(String, f64)> = Vec::new();

    for r in results.iter().filter(|r| r.success) {
        // 记录历史
        history_records.push(HistoryRecord {
            timestamp: now,
            domain: r.endpoint.domain.clone(),
            original_latency: r.original_latency,
            optimized_latency: r.latency,
            speedup_percent: r.speedup_percent,
            applied: true, // 总是应用
        });

        // 总是绑定最优 IP（r.ip 已经是最优的了）
        bindings.push(HostsBinding {
            domain: r.endpoint.domain.clone(),
            ip: r.ip.clone(),
        });

        // 收集基准延迟更新
        baseline_updates.push((r.endpoint.domain.clone(), r.latency));
    }

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

// ===== 自动模式命令 =====

#[tauri::command]
async fn start_auto_mode(state: State<'_, AppState>, app_handle: AppHandle) -> Result<(), String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;

    // 原子检查并设置（单次锁操作避免竞态条件）
    let cancel_token = {
        let mut token = state.auto_mode_token.lock().await;
        if token.is_some() {
            return Err("自动模式已在运行".into());
        }
        let new_token = CancellationToken::new();
        *token = Some(new_token.clone());
        new_token
    };

    // 克隆需要的数据
    let checker = state.health_checker.clone();
    let config_clone = config.clone();
    let auto_mode_token = state.auto_mode_token.clone();

    // 启动后台任务
    tauri::async_runtime::spawn(async move {
        // 重置 health_checker 的取消令牌
        {
            let mut checker_guard = checker.lock().await;
            checker_guard.reset_cancel_token();
        }

        // start() 是同步的，在内部 spawn 任务
        {
            let checker_guard = checker.lock().await;
            checker_guard.start(app_handle, config_clone);
        }

        // 等待取消信号
        cancel_token.cancelled().await;

        // 任务结束时清除 auto_mode_token
        {
            let mut token = auto_mode_token.lock().await;
            *token = None;
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_auto_mode(state: State<'_, AppState>) -> Result<(), String> {
    let mut token = state.auto_mode_token.lock().await;
    if let Some(t) = token.take() {
        t.cancel();

        // 同时取消 health_checker 的令牌
        let checker = state.health_checker.lock().await;
        checker.get_cancel_token().cancel();
    }
    Ok(())
}

#[tauri::command]
async fn get_auto_mode_status(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    let checker = state.health_checker.lock().await;
    Ok(checker.get_status().await)
}

#[tauri::command]
async fn is_auto_mode_running(state: State<'_, AppState>) -> Result<bool, String> {
    let token = state.auto_mode_token.lock().await;
    Ok(token.is_some())
}

// ===== 简化工作流命令 =====

/// 启动工作流：测速 + 应用 + 启动健康检查
/// Requirements: 3.1, 3.2, 3.3
#[tauri::command]
async fn start_workflow(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<WorkflowResult, String> {
    // Step 1: 加载配置并获取启用的端点 (Requirement 3.1)
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let endpoints: Vec<Endpoint> = config
        .endpoints
        .iter()
        .filter(|e| e.enabled)
        .cloned()
        .collect();

    if endpoints.is_empty() {
        return Err("没有启用的端点".into());
    }

    let test_count = endpoints.len() as u32;

    // Step 2: 执行测速 (Requirement 3.1)
    let tester = EndpointTester::new(vec![]);

    // 保存 tester 以便取消
    {
        let mut t = state.tester.lock().await;
        *t = Some(tester.clone());
    }

    // 使用全局超时（60秒）防止永久卡住
    let test_future = tester.test_all(&endpoints);
    let results = match tokio::time::timeout(std::time::Duration::from_secs(60), test_future).await
    {
        Ok(results) => results,
        Err(_) => {
            // 超时，取消测试
            tester.cancel();
            // 清除 tester
            let mut t = state.tester.lock().await;
            *t = None;
            return Err("测速超时（60秒），请检查网络连接".into());
        }
    };

    // 清除 tester
    {
        let mut t = state.tester.lock().await;
        *t = None;
    }

    // 更新基准延迟
    let baselines = {
        let checker = state.health_checker.lock().await;
        checker.get_baselines_arc()
    };

    for r in &results {
        if r.success {
            let mut b = baselines.lock().await;
            b.insert(r.endpoint.domain.clone(), r.latency);
        }
    }

    // 保存结果到状态
    {
        let mut state_results = state.results.lock().await;
        *state_results = results.clone();
    }

    // Step 3: 应用所有成功的端点到 hosts 文件 (Requirement 3.2)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut bindings: Vec<HostsBinding> = Vec::new();
    let mut history_records: Vec<HistoryRecord> = Vec::new();
    let mut baseline_updates: Vec<(String, f64)> = Vec::new();
    let mut success_count = 0u32;

    for r in results.iter().filter(|r| r.success) {
        success_count += 1;

        // 记录历史
        history_records.push(HistoryRecord {
            timestamp: now,
            domain: r.endpoint.domain.clone(),
            original_latency: r.original_latency,
            optimized_latency: r.latency,
            speedup_percent: r.speedup_percent,
            applied: true,
        });

        // 绑定最优 IP
        bindings.push(HostsBinding {
            domain: r.endpoint.domain.clone(),
            ip: r.ip.clone(),
        });

        // 收集基准延迟更新
        baseline_updates.push((r.endpoint.domain.clone(), r.latency));
    }

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

    // 应用绑定
    let applied_count = if !bindings.is_empty() {
        let count = hosts_ops::write_bindings_batch(&bindings).map_err(|e| e.to_string())?;
        hosts_ops::flush_dns().map_err(|e| e.to_string())?;
        count as u32
    } else {
        0
    };

    // Step 4: 启动健康检查任务 (Requirement 3.3)
    // 检查是否已有运行中的健康检查任务
    let already_running = {
        let token = state.auto_mode_token.lock().await;
        token.is_some()
    };

    if !already_running && success_count > 0 {
        let cancel_token = CancellationToken::new();
        {
            let mut token = state.auto_mode_token.lock().await;
            *token = Some(cancel_token.clone());
        }

        // 克隆需要的数据
        let checker = state.health_checker.clone();
        let config_clone = config.clone();
        let auto_mode_token = state.auto_mode_token.clone();

        // 启动后台健康检查任务
        tauri::async_runtime::spawn(async move {
            // 重置 health_checker 的取消令牌
            {
                let mut checker_guard = checker.lock().await;
                checker_guard.reset_cancel_token();
            }

            // start() 是同步的，在内部 spawn 任务
            {
                let checker_guard = checker.lock().await;
                checker_guard.start(app_handle, config_clone);
            }

            // 等待取消信号
            cancel_token.cancelled().await;

            // 任务结束时清除 auto_mode_token
            {
                let mut token = auto_mode_token.lock().await;
                *token = None;
            }
        });
    }

    Ok(WorkflowResult {
        test_count,
        success_count,
        applied_count,
        results,
    })
}

/// 获取工作流状态
/// Requirements: 5.4
#[tauri::command]
async fn is_workflow_running(state: State<'_, AppState>) -> Result<bool, String> {
    let token = state.auto_mode_token.lock().await;
    Ok(token.is_some())
}

/// 获取当前测速结果
/// 用于程序启动时恢复已有的测速数据
#[tauri::command]
async fn get_current_results(state: State<'_, AppState>) -> Result<Vec<EndpointResult>, String> {
    let results = state.results.lock().await;
    Ok(results.clone())
}

/// 停止工作流：停止健康检查 + 清除 hosts
/// Requirements: 4.1, 4.2, 4.3
#[tauri::command]
async fn stop_workflow(state: State<'_, AppState>) -> Result<u32, String> {
    // Step 1: 停止健康检查任务 (Requirement 4.1)
    {
        let mut token = state.auto_mode_token.lock().await;
        if let Some(t) = token.take() {
            t.cancel();

            // 同时取消 health_checker 的令牌
            let checker = state.health_checker.lock().await;
            checker.get_cancel_token().cancel();
        }
    }

    // Step 2: 清除所有 anyFAST 管理的 hosts 绑定 (Requirement 4.2)
    // 使用 clear_all_anyfast_bindings 清除整个 anyFAST 块，
    // 而不是只清除当前配置中的域名，这样可以确保清除所有历史绑定
    let count = hosts_ops::clear_all_anyfast_bindings().map_err(|e| e.to_string())?;

    // Step 3: 刷新 DNS 缓存 (Requirement 4.3)
    // 即使没有清除任何绑定，也刷新 DNS 以确保状态一致
    hosts_ops::flush_dns().map_err(|e| e.to_string())?;

    Ok(count as u32)
}

// 当前版本号（从 Cargo.toml 读取）
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        .setup(|app| {
            let config_manager = ConfigManager::new();

            let state = AppState {
                config_manager: config_manager.clone(),
                history_manager: HistoryManager::new(),
                tester: Arc::new(Mutex::new(None)),
                results: Arc::new(Mutex::new(Vec::new())),
                health_checker: Arc::new(Mutex::new(HealthChecker::new(config_manager.clone()))),
                auto_mode_token: Arc::new(Mutex::new(None)),
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
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            // 退出前始终清除 hosts（强制行为）
                            let _ = hosts_ops::clear_all_anyfast_bindings();
                            let _ = hosts_ops::flush_dns();
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
                            let _ = window.show();
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
                            let _ = win.hide();
                        }
                    }
                });
            }

            // 自动启动健康检查
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 延迟 2 秒启动，等待应用完全初始化
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // 调用 start_auto_mode
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let config = state.config_manager.load().ok();
                    if let Some(config) = config {
                        let cancel_token = CancellationToken::new();
                        {
                            let mut token = state.auto_mode_token.lock().await;
                            *token = Some(cancel_token.clone());
                        }

                        // start() 现在是同步的，在内部 spawn 任务
                        let checker = state.health_checker.lock().await;
                        checker.start(app_handle.clone(), config);
                        // 锁在这里立即释放
                    }
                }
            });

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
            get_bindings,
            get_binding_count,
            check_admin,
            is_service_running,
            get_permission_status,
            refresh_service_status,
            get_hosts_path,
            open_hosts_file,
            get_history_stats,
            clear_history,
            // 自动模式
            start_auto_mode,
            stop_auto_mode,
            get_auto_mode_status,
            is_auto_mode_running,
            // 简化工作流
            start_workflow,
            stop_workflow,
            is_workflow_running,
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
