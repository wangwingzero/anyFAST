//! 健康检查模块
//! 基准延迟跟踪 + 持续优化后台任务

use crate::config::ConfigManager;
use crate::endpoint_tester::EndpointTester;
use crate::hosts_manager::HostsBinding;
use crate::hosts_ops;
use crate::models::{Endpoint, EndpointResult, OptimizationEvent, OptimizationEventType};
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(feature = "tauri-runtime")]
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// 基准延迟跟踪器
/// 记录每个域名的基准延迟，用于测速结果展示
pub struct BaselineTracker {
    baselines: Arc<Mutex<HashMap<String, f64>>>,
}

impl BaselineTracker {
    pub fn new() -> Self {
        Self {
            baselines: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取 baselines 的 Arc 克隆（避免长时间持有锁）
    pub fn get_baselines_arc(&self) -> Arc<Mutex<HashMap<String, f64>>> {
        self.baselines.clone()
    }
}

/// 持续优化后台任务
pub struct HealthChecker {
    cancel_token: CancellationToken,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

/// 记录每个域名当前 IP 连续失败的次数
type FailureCounter = HashMap<String, u32>;

impl HealthChecker {
    /// 启动持续优化后台任务
    #[cfg(feature = "tauri-runtime")]
    pub fn start(
        app_handle: AppHandle,
        config_manager: ConfigManager,
        results: Arc<Mutex<Vec<EndpointResult>>>,
        baselines: Arc<Mutex<HashMap<String, f64>>>,
    ) -> Self {
        let cancel_token = CancellationToken::new();
        let token = cancel_token.clone();

        let task_handle = tokio::spawn(async move {
            Self::run_loop(app_handle, config_manager, results, baselines, token).await;
        });

        Self {
            cancel_token,
            task_handle: Some(task_handle),
        }
    }

    /// 停止后台任务（带 10 秒超时保护，防止永久阻塞）
    pub async fn stop(&mut self) {
        self.cancel_token.cancel();
        if let Some(handle) = self.task_handle.take() {
            let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), handle);
            match timeout.await {
                Ok(_) => {}
                Err(_) => {
                    eprintln!("HealthChecker: stop() 超时，后台任务可能仍在运行");
                }
            }
        }
    }

    /// 查询是否正在运行
    pub fn is_running(&self) -> bool {
        if let Some(handle) = &self.task_handle {
            !handle.is_finished()
        } else {
            false
        }
    }

    /// 核心循环
    #[cfg(feature = "tauri-runtime")]
    async fn run_loop(
        app_handle: AppHandle,
        config_manager: ConfigManager,
        results: Arc<Mutex<Vec<EndpointResult>>>,
        baselines: Arc<Mutex<HashMap<String, f64>>>,
        cancel_token: CancellationToken,
    ) {
        // 通知前端已启动
        let _ = app_handle.emit(
            "optimization-event",
            OptimizationEvent {
                event_type: OptimizationEventType::Started,
                message: "持续优化已启动".into(),
                ..Default::default()
            },
        );

        // 连续失败计数器：域名 → 连续失败次数
        let mut failure_counts: FailureCounter = HashMap::new();

        // 跨循环复用 EndpointTester（TLS connector + DNS resolver 开销大）
        let mut cached_tester: Option<EndpointTester> = None;
        let mut cached_preferred_ips: Vec<String> = Vec::new();
        let mut cached_test_count: u32 = 0;

        // 全量优选冷却期追踪：域名 → 上次全量优选时间
        let mut last_full_test: HashMap<String, std::time::Instant> = HashMap::new();
        const FULL_TEST_COOLDOWN: std::time::Duration = std::time::Duration::from_secs(600); // 10 分钟

        loop {
            // 每次循环开始重新加载 config
            let config = match config_manager.load() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("HealthChecker: 加载配置失败: {}", e);
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => continue,
                        _ = cancel_token.cancelled() => break,
                    }
                }
            };

            // 如果 continuous_mode 被关闭，退出循环
            if !config.continuous_mode {
                break;
            }

            let interval = std::time::Duration::from_secs(config.check_interval);

            // 等待检查间隔或取消信号
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = cancel_token.cancelled() => break,
            }

            if cancel_token.is_cancelled() {
                break;
            }

            // 找出已绑定的端点
            let bound_endpoints: Vec<(Endpoint, String)> = config
                .endpoints
                .iter()
                .filter_map(|ep| hosts_ops::read_binding(&ep.domain).map(|ip| (ep.clone(), ip)))
                .collect();

            if bound_endpoints.is_empty() {
                continue;
            }

            // 复用 EndpointTester：仅在配置变化时重建
            let tester = match &cached_tester {
                Some(t)
                    if cached_preferred_ips == config.preferred_ips
                        && cached_test_count == config.test_count =>
                {
                    t.clone()
                }
                _ => {
                    let t = EndpointTester::new(config.preferred_ips.clone(), config.test_count);
                    cached_preferred_ips = config.preferred_ips.clone();
                    cached_test_count = config.test_count;
                    cached_tester = Some(t.clone());
                    t
                }
            };

            // === Phase 1: 轻量级检查 — 仅测当前绑定 IP（每端点 1 次 TLS 连接） ===
            let mut join_set = tokio::task::JoinSet::new();
            for (ep, current_ip) in &bound_endpoints {
                let tester_clone = tester.clone();
                let ep_clone = ep.clone();
                let current_ip_clone = current_ip.clone();
                join_set.spawn(async move {
                    let current_result =
                        tester_clone.test_ip(&ep_clone, current_ip_clone.clone()).await;
                    (ep_clone, current_ip_clone, current_result)
                });
            }

            // 收集轻量级检查结果
            let mut light_results: Vec<(Endpoint, String, EndpointResult)> = Vec::new();
            while let Some(result) = join_set.join_next().await {
                if cancel_token.is_cancelled() {
                    break;
                }
                let Ok(item) = result else { continue };
                light_results.push(item);
            }

            if cancel_token.is_cancelled() {
                break;
            }

            // === Phase 2: 判断哪些端点需要全量优选 ===
            let baselines_snapshot = baselines.lock().await.clone();
            let mut needs_full_test: Vec<(Endpoint, String)> = Vec::new();

            for (ep, current_ip, current_result) in &light_results {
                if current_result.success {
                    // 当前 IP 成功 — 重置失败计数
                    failure_counts.remove(&ep.domain);

                    // 检查延迟是否严重恶化（比基准高 slow_threshold% 且绝对增加超 300ms）
                    if let Some(&baseline) = baselines_snapshot.get(&ep.domain) {
                        if baseline > 0.0 {
                            let threshold_latency =
                                baseline * (1.0 + config.slow_threshold as f64 / 100.0);
                            let abs_increase = current_result.latency - baseline;
                            if current_result.latency > threshold_latency && abs_increase > 300.0 {
                                needs_full_test.push((ep.clone(), current_ip.clone()));
                            }
                        }
                    }
                } else {
                    // 当前 IP 失败 — 累加失败计数
                    let count = failure_counts.entry(ep.domain.clone()).or_insert(0);
                    *count += 1;
                    if *count >= config.failure_threshold {
                        needs_full_test.push((ep.clone(), current_ip.clone()));
                    }
                }
            }

            // 应用冷却期过滤：每个域名全量优选后 10 分钟内不重复触发
            let now = std::time::Instant::now();
            needs_full_test.retain(|(ep, _)| match last_full_test.get(&ep.domain) {
                Some(last_time) => now.duration_since(*last_time) >= FULL_TEST_COOLDOWN,
                None => true,
            });

            // === Phase 3: 对需要全量优选的端点执行 test_endpoint ===
            struct SwitchAction {
                domain: String,
                old_ip: String,
                new_ip: String,
                old_latency: Option<f64>,
                new_latency: f64,
                best_result: EndpointResult,
            }

            let mut switch_actions: Vec<SwitchAction> = Vec::new();

            if !needs_full_test.is_empty() {
                let mut full_join_set = tokio::task::JoinSet::new();
                for (ep, current_ip) in &needs_full_test {
                    // 记录全量优选时间（冷却期起点）
                    last_full_test.insert(ep.domain.clone(), now);

                    let tester_clone = tester.clone();
                    let ep_clone = ep.clone();
                    let current_ip_clone = current_ip.clone();
                    full_join_set.spawn(async move {
                        let best_result = tester_clone.test_endpoint(&ep_clone).await;
                        (ep_clone, current_ip_clone, best_result)
                    });
                }

                while let Some(result) = full_join_set.join_next().await {
                    if cancel_token.is_cancelled() {
                        break;
                    }
                    let Ok((ep, current_ip, best_result)) = result else {
                        continue;
                    };

                    if !best_result.success {
                        continue;
                    }

                    let new_ip = &best_result.ip;
                    let new_latency = best_result.latency;

                    // 从轻量检查结果获取当前延迟
                    let current_latency = light_results
                        .iter()
                        .find(|item| item.0.domain == ep.domain)
                        .and_then(|item| {
                            if item.2.success {
                                Some(item.2.latency)
                            } else {
                                None
                            }
                        });

                    // 同 IP 跳过
                    if new_ip == &current_ip {
                        failure_counts.remove(&ep.domain);
                        continue;
                    }

                    let should_switch = if let Some(cur_lat) = current_latency {
                        // 当前 IP 能通但延迟恶化 — 需要明显更好才切换
                        if cur_lat <= 0.0 {
                            false
                        } else {
                            let improvement_pct =
                                (cur_lat - new_latency) / cur_lat * 100.0;
                            let improvement_abs = cur_lat - new_latency;
                            improvement_pct > 20.0 && improvement_abs > 50.0
                        }
                    } else {
                        // 当前 IP 不可达 — 有可用候选就切换
                        failure_counts.remove(&ep.domain);
                        true
                    };

                    if should_switch {
                        switch_actions.push(SwitchAction {
                            domain: ep.domain.clone(),
                            old_ip: current_ip.clone(),
                            new_ip: new_ip.clone(),
                            old_latency: current_latency,
                            new_latency,
                            best_result,
                        });
                    }
                }
            }

            if cancel_token.is_cancelled() {
                break;
            }

            // 批量执行切换：一次性写入所有变更，只 flush DNS 一次
            let switched_count = if !switch_actions.is_empty() {
                let bindings: Vec<HostsBinding> = switch_actions
                    .iter()
                    .map(|a| HostsBinding {
                        domain: a.domain.clone(),
                        ip: a.new_ip.clone(),
                    })
                    .collect();

                match hosts_ops::write_bindings_batch(&bindings) {
                    Ok(count) => {
                        if count > 0 {
                            let _ = hosts_ops::flush_dns();
                        }

                        // 批量更新状态 + 通知前端
                        // 先批量更新 baselines（只获取一次锁）
                        {
                            let mut b = baselines.lock().await;
                            for action in &switch_actions {
                                b.insert(action.domain.clone(), action.new_latency);
                            }
                        }

                        // 再批量更新 results（只获取一次锁）
                        {
                            let mut state_results = results.lock().await;
                            for action in &switch_actions {
                                if let Some(existing) = state_results
                                    .iter_mut()
                                    .find(|r| r.endpoint.domain == action.domain)
                                {
                                    *existing = action.best_result.clone();
                                }
                            }
                        }

                        // 通知前端每个切换事件
                        for action in &switch_actions {
                            failure_counts.remove(&action.domain);

                            let _ = app_handle.emit(
                                "optimization-event",
                                OptimizationEvent {
                                    event_type: OptimizationEventType::AutoSwitch,
                                    domain: Some(action.domain.clone()),
                                    old_ip: Some(action.old_ip.clone()),
                                    new_ip: Some(action.new_ip.clone()),
                                    old_latency: action.old_latency,
                                    new_latency: Some(action.new_latency),
                                    message: format!(
                                        "{} 已自动切换: {} → {} ({:.0}ms → {:.0}ms)",
                                        action.domain,
                                        action.old_ip,
                                        action.new_ip,
                                        action.old_latency.unwrap_or(9999.0),
                                        action.new_latency,
                                    ),
                                },
                            );
                        }

                        count
                    }
                    Err(e) => {
                        eprintln!("HealthChecker: 批量写入绑定失败: {}", e);
                        0
                    }
                }
            } else {
                0
            };

            // 通知前端本轮检查完成
            let _ = app_handle.emit(
                "optimization-event",
                OptimizationEvent {
                    event_type: OptimizationEventType::CheckComplete,
                    message: format!(
                        "健康检查完成: 检测 {} 个端点，切换 {} 个",
                        bound_endpoints.len(),
                        switched_count
                    ),
                    ..Default::default()
                },
            );
        }

        // 通知前端已停止
        let _ = app_handle.emit(
            "optimization-event",
            OptimizationEvent {
                event_type: OptimizationEventType::Stopped,
                message: "持续优化已停止".into(),
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_baseline_tracker_new() {
        let tracker = BaselineTracker::new();
        let baselines = tracker.get_baselines_arc();
        let b = baselines.lock().await;
        assert!(b.is_empty());
    }

    #[tokio::test]
    async fn test_baseline_tracker_set_and_get() {
        let tracker = BaselineTracker::new();
        let baselines = tracker.get_baselines_arc();
        {
            let mut b = baselines.lock().await;
            b.insert("test.com".to_string(), 100.0);
        }
        let b = baselines.lock().await;
        assert_eq!(b.get("test.com"), Some(&100.0));
    }
}
