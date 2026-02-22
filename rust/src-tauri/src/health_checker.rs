//! 健康检查模块
//! 基准延迟跟踪 + 持续优化后台任务

use crate::config::ConfigManager;
use crate::endpoint_tester::EndpointTester;
use crate::hosts_manager::HostsBinding;
use crate::hosts_ops;
use crate::models::{Endpoint, EndpointResult, OptimizationEvent, OptimizationEventType};
use std::collections::HashMap;
use std::sync::Arc;
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

            // 轻量级测速：仅测当前绑定 IP + 用 test_endpoint 找最优候选
            // 并发测试所有绑定端点（每个端点内部已经是并发测 IP 的）
            let mut join_set = tokio::task::JoinSet::new();
            for (ep, current_ip) in &bound_endpoints {
                let tester_clone = tester.clone();
                let ep_clone = ep.clone();
                let current_ip_clone = current_ip.clone();
                join_set.spawn(async move {
                    // 并发执行：1) 测当前绑定 IP  2) 找最优候选
                    let current_test = tester_clone.test_ip(&ep_clone, current_ip_clone.clone());
                    let best_test = tester_clone.test_endpoint(&ep_clone);
                    let (current_result, best_result) = tokio::join!(current_test, best_test);
                    (ep_clone, current_ip_clone, current_result, best_result)
                });
            }

            // 收集结果
            struct SwitchAction {
                domain: String,
                old_ip: String,
                new_ip: String,
                old_latency: Option<f64>,
                new_latency: f64,
                best_result: EndpointResult,
            }

            let mut switch_actions: Vec<SwitchAction> = Vec::new();

            while let Some(result) = join_set.join_next().await {
                if cancel_token.is_cancelled() {
                    break;
                }
                let Ok((ep, current_ip, current_result, best_result)) = result else {
                    continue;
                };

                if !best_result.success {
                    continue;
                }

                let new_ip = &best_result.ip;
                let new_latency = best_result.latency;
                let current_latency = if current_result.success {
                    Some(current_result.latency)
                } else {
                    None
                };

                // 同 IP 跳过
                if new_ip == &current_ip {
                    failure_counts.remove(&ep.domain);
                    continue;
                }

                let should_switch = if let Some(cur_lat) = current_latency {
                    failure_counts.remove(&ep.domain);
                    if cur_lat <= 0.0 {
                        // 延迟为零或负值，视为异常数据，不切换
                        false
                    } else {
                        let improvement_pct = (cur_lat - new_latency) / cur_lat * 100.0;
                        let improvement_abs = cur_lat - new_latency;
                        improvement_pct > 20.0 && improvement_abs > 50.0
                    }
                } else {
                    let count = failure_counts.entry(ep.domain.clone()).or_insert(0);
                    *count += 1;
                    *count >= config.failure_threshold
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
