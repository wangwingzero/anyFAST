//! AnyRouter FAST - Rust Backend
//! 中转站端点优选工具

mod config;
mod endpoint_tester;
mod health_checker;
mod history;
mod hosts_manager;
mod models;

use config::ConfigManager;
use endpoint_tester::EndpointTester;
use health_checker::{HealthChecker, HealthStatus};
use history::HistoryManager;
use hosts_manager::{HostsBinding, HostsManager};
use models::{AppConfig, Endpoint, EndpointResult, HistoryRecord, HistoryStats};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    AppHandle, Manager, State,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
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
    state.config_manager.save(&config).map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_speed_test(state: State<'_, AppState>) -> Result<Vec<EndpointResult>, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let endpoints: Vec<Endpoint> = config.endpoints.into_iter().filter(|e| e.enabled).collect();

    if endpoints.is_empty() {
        return Err("没有启用的端点".into());
    }

    let tester = EndpointTester::new(config.cloudflare_ips);

    // 保存 tester 以便取消
    {
        let mut t = state.tester.lock().await;
        *t = Some(tester.clone());
    }

    // 使用全局超时（60秒）防止永久卡住
    let test_future = tester.test_all(&endpoints);
    let results = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        test_future
    ).await {
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
    HostsManager::write_binding(&domain, &ip).map_err(|e| e.to_string())?;
    HostsManager::flush_dns().map_err(|e| e.to_string())
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
    let count = HostsManager::write_bindings_batch(&bindings).map_err(|e| e.to_string())?;
    HostsManager::flush_dns().map_err(|e| e.to_string())?;

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
    let count = HostsManager::clear_bindings_batch(&domains).map_err(|e| e.to_string())?;

    if count > 0 {
        HostsManager::flush_dns().map_err(|e| e.to_string())?;
    }

    Ok(count as u32)
}

#[tauri::command]
async fn get_bindings(state: State<'_, AppState>) -> Result<Vec<(String, Option<String>)>, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let mut bindings = Vec::new();

    for endpoint in config.endpoints {
        let ip = HostsManager::read_binding(&endpoint.domain);
        bindings.push((endpoint.domain, ip));
    }

    Ok(bindings)
}

#[tauri::command]
async fn get_binding_count(state: State<'_, AppState>) -> Result<u32, String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;
    let mut count = 0;

    for endpoint in config.endpoints {
        if HostsManager::read_binding(&endpoint.domain).is_some() {
            count += 1;
        }
    }

    Ok(count)
}

#[tauri::command]
fn check_admin() -> bool {
    #[cfg(windows)]
    {
        // Simple check: try to open hosts file for write
        use std::fs::OpenOptions;
        let path = r"C:\Windows\System32\drivers\etc\hosts";
        OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .is_ok()
    }

    #[cfg(not(windows))]
    {
        // On Unix, check if running as root
        unsafe { libc::geteuid() == 0 }
    }
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
    state.history_manager.get_stats(hours).map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history_manager.clear_all().map_err(|e| e.to_string())
}

// ===== 自动模式命令 =====

#[tauri::command]
async fn start_auto_mode(
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let config = state.config_manager.load().map_err(|e| e.to_string())?;

    if config.mode != "auto" {
        return Err("当前不是自动模式，请在设置中切换".into());
    }

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let config_manager = ConfigManager::new();

            // 检查是否需要自动启动
            let should_auto_start = config_manager
                .load()
                .map(|c| c.mode == "auto")
                .unwrap_or(false);

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
                            // 退出前检查是否需要清除 hosts
                            if let Some(state) = app.try_state::<AppState>() {
                                if let Ok(config) = state.config_manager.load() {
                                    if config.clear_on_exit {
                                        let domains: Vec<&str> = config.endpoints.iter()
                                            .map(|e| e.domain.as_str())
                                            .collect();
                                        let _ = HostsManager::clear_bindings_batch(&domains);
                                        let _ = HostsManager::flush_dns();
                                    }
                                }
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    // 左键点击托盘图标显示窗口
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // 处理窗口关闭事件
            let app_handle = app.handle().clone();
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        // 检查配置是否最小化到托盘
                        if let Some(state) = app_handle.try_state::<AppState>() {
                            if let Ok(config) = state.config_manager.load() {
                                if config.close_to_tray {
                                    // 阻止关闭，改为隐藏窗口
                                    api.prevent_close();
                                    if let Some(win) = app_handle.get_webview_window("main") {
                                        let _ = win.hide();
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // 如果配置为自动模式，启动时自动开始健康检查
            if should_auto_start {
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // 延迟 2 秒启动，等待应用完全初始化
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                    // 调用 start_auto_mode
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        let config = state.config_manager.load().ok();
                        if let Some(config) = config {
                            if config.mode == "auto" {
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
            get_bindings,
            get_binding_count,
            check_admin,
            get_hosts_path,
            open_hosts_file,
            get_history_stats,
            clear_history,
            // 自动模式
            start_auto_mode,
            stop_auto_mode,
            get_auto_mode_status,
            is_auto_mode_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
