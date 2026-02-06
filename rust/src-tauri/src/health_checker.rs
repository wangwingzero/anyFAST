//! 后台健康检查模块
//! 定期检测当前绑定的 hosts 是否正常工作

use crate::config::ConfigManager;
use crate::endpoint_tester::EndpointTester;
use crate::hosts_manager::HostsBinding;
use crate::hosts_ops;
use crate::models::{AppConfig, Endpoint};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

/// 健康检查状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub is_running: bool,
    pub last_check: Option<i64>,
    pub check_count: u32,
    pub switch_count: u32,
    pub endpoints_status: Vec<EndpointHealth>,
}

/// 单个端点的健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointHealth {
    pub domain: String,
    pub current_ip: Option<String>,
    pub best_ip: Option<String>, // 当前最优 IP
    pub best_latency: f64,       // 最优 IP 的延迟
    pub latency: f64,
    pub baseline_latency: f64,
    pub consecutive_failures: u32,
    pub is_healthy: bool,
    pub recommend_retest: bool,
}

const FAILURE_WINDOW_SIZE: usize = 10;
const FAILURE_WINDOW_THRESHOLD: usize = 7;
const SEVERE_WINDOW_SIZE: usize = 5;
const SEVERE_WINDOW_THRESHOLD: usize = 3;
const SWITCH_COOLDOWN_SECS: i64 = 30 * 60;
const SWITCH_SILENT_WINDOW_SECS: i64 = 120;
const SEVERE_ABS_THRESHOLD_MS: f64 = 300.0;
const MIN_CHECK_INTERVAL_SECS: u64 = 60;
const MIN_SLOW_THRESHOLD_PERCENT: u32 = 100;
const MIN_FAILURE_THRESHOLD: u32 = 3;

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 健康检查器
pub struct HealthChecker {
    config_manager: ConfigManager,
    cancel_token: CancellationToken,
    status: Arc<Mutex<HealthStatus>>,
    /// 基准延迟记录 (domain -> baseline_latency)
    baselines: Arc<Mutex<HashMap<String, f64>>>,
}

impl HealthChecker {
    pub fn new(config_manager: ConfigManager) -> Self {
        Self {
            config_manager,
            cancel_token: CancellationToken::new(),
            status: Arc::new(Mutex::new(HealthStatus {
                is_running: false,
                last_check: None,
                check_count: 0,
                switch_count: 0,
                endpoints_status: Vec::new(),
            })),
            baselines: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 获取取消令牌（用于停止）
    pub fn get_cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    /// 重置取消令牌（用于重新启动）
    pub fn reset_cancel_token(&mut self) {
        self.cancel_token = CancellationToken::new();
    }

    /// 获取当前状态
    pub async fn get_status(&self) -> HealthStatus {
        self.status.lock().await.clone()
    }

    /// 设置基准延迟
    #[allow(dead_code)]
    pub async fn set_baseline(&self, domain: &str, latency: f64) {
        let mut baselines = self.baselines.lock().await;
        baselines.insert(domain.to_string(), latency);
    }

    /// 获取 baselines 的 Arc 克隆（用于避免长时间持有 HealthChecker 锁）
    pub fn get_baselines_arc(&self) -> Arc<Mutex<HashMap<String, f64>>> {
        self.baselines.clone()
    }

    fn dedupe_endpoints_by_domain(endpoints: Vec<Endpoint>) -> Vec<Endpoint> {
        let mut seen = HashSet::new();
        let mut deduped = Vec::new();

        for endpoint in endpoints {
            if seen.insert(endpoint.domain.clone()) {
                deduped.push(endpoint);
            }
        }

        deduped
    }

    /// 静默窗口判定：
    /// - 首次触发仅记录时间，不切换
    /// - 静默窗口未到不切换
    /// - 条件恢复则清除待切换计时
    /// - 静默窗口到达后，还需通过冷却窗口检查
    fn should_switch_after_silent_window(
        domain: &str,
        now: i64,
        should_switch_condition: bool,
        in_cooldown: bool,
        pending_switch_since: &mut HashMap<String, i64>,
    ) -> bool {
        if !should_switch_condition {
            pending_switch_since.remove(domain);
            return false;
        }

        let since = pending_switch_since
            .entry(domain.to_string())
            .or_insert(now);
        let silent_elapsed = now.saturating_sub(*since);
        if silent_elapsed < SWITCH_SILENT_WINDOW_SECS {
            return false;
        }

        !in_cooldown
    }

    /// 启动后台健康检查（在内部 spawn 任务，立即返回）
    pub fn start(&self, app_handle: AppHandle, config: AppConfig) {
        let cancel_token = self.cancel_token.clone();
        let status = self.status.clone();
        let baselines = self.baselines.clone();
        let config_manager = self.config_manager.clone();

        let check_interval = config.check_interval.max(MIN_CHECK_INTERVAL_SECS);
        let slow_threshold = config.slow_threshold.max(MIN_SLOW_THRESHOLD_PERCENT);
        let failure_threshold = config.failure_threshold.max(MIN_FAILURE_THRESHOLD);

        // 获取启用的端点（按 domain 去重，避免重复 domain 导致过度触发切换）
        let endpoints = Self::dedupe_endpoints_by_domain(
            config.endpoints.into_iter().filter(|e| e.enabled).collect(),
        );

        if endpoints.is_empty() {
            return;
        }

        // 在独立任务中运行，立即返回
        tokio::spawn(async move {
            // 标记为运行中
            {
                let mut s = status.lock().await;
                s.is_running = true;
            }

            // 通知前端
            let _ = app_handle.emit("health-status-changed", true);

            // 连续失败计数器
            let failure_counts: Arc<Mutex<HashMap<String, u32>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let failure_windows: Arc<Mutex<HashMap<String, VecDeque<bool>>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let severe_windows: Arc<Mutex<HashMap<String, VecDeque<bool>>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let last_switch_times: Arc<Mutex<HashMap<String, i64>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let pending_switch_since: Arc<Mutex<HashMap<String, i64>>> =
                Arc::new(Mutex::new(HashMap::new()));

            // 主循环
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        // 收到取消信号，退出循环
                        break;
                    }
                    _ = sleep(Duration::from_secs(check_interval)) => {
                        // 执行健康检查
                        let check_result = Self::perform_check(
                            &endpoints,
                            &baselines,
                            &failure_counts,
                            &failure_windows,
                            &severe_windows,
                            &last_switch_times,
                            &pending_switch_since,
                            slow_threshold,
                            failure_threshold,
                        ).await;

                        // 更新状态
                        {
                            let mut s = status.lock().await;
                            s.last_check = Some(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(0)
                            );
                            s.check_count += 1;
                            s.endpoints_status = check_result.endpoints_health.clone();
                        }

                        // 发送检查结果到前端
                        let _ = app_handle.emit("health-check-result", &check_result);

                        // 如果需要切换，执行切换
                        if !check_result.needs_switch.is_empty() {
                            let switch_result = Self::perform_switch(
                                &check_result.needs_switch,
                                &baselines,
                                &last_switch_times,
                                &config_manager,
                            ).await;

                            if switch_result.switched_count > 0 {
                                let mut s = status.lock().await;
                                s.switch_count += switch_result.switched_count;

                                // 通知前端
                                let _ = app_handle.emit("auto-switch", &switch_result);
                            }
                        }
                    }
                }
            }

            // 标记为停止
            {
                let mut s = status.lock().await;
                s.is_running = false;
            }
            let _ = app_handle.emit("health-status-changed", false);
        });
    }

    /// 执行健康检查
    async fn perform_check(
        endpoints: &[Endpoint],
        baselines: &Arc<Mutex<HashMap<String, f64>>>,
        failure_counts: &Arc<Mutex<HashMap<String, u32>>>,
        failure_windows: &Arc<Mutex<HashMap<String, VecDeque<bool>>>>,
        severe_windows: &Arc<Mutex<HashMap<String, VecDeque<bool>>>>,
        last_switch_times: &Arc<Mutex<HashMap<String, i64>>>,
        pending_switch_since: &Arc<Mutex<HashMap<String, i64>>>,
        slow_threshold: u32,
        failure_threshold: u32,
    ) -> CheckResult {
        let tester = EndpointTester::new(vec![]);
        let mut endpoints_health = Vec::new();
        let mut needs_switch = Vec::new();
        let now = current_timestamp();

        for endpoint in endpoints {
            // 获取当前绑定的 IP
            let current_ip = hosts_ops::read_binding(&endpoint.domain);

            // 测试端点（会测试所有 Cloudflare IP 并返回最优结果）
            let result = tester.test_endpoint(endpoint).await;

            // 测试当前绑定 IP（用于确认是否失效/明显变慢）
            let (current_success, current_latency) = if let Some(current) = current_ip.as_ref() {
                if result.success && current == &result.ip {
                    (true, result.latency)
                } else {
                    let current_result = tester.test_ip(endpoint, current.clone()).await;
                    (current_result.success, current_result.latency)
                }
            } else {
                (result.success, result.latency)
            };

            // 获取基准延迟
            let baseline = {
                let b = baselines.lock().await;
                b.get(&endpoint.domain)
                    .copied()
                    .unwrap_or(if current_latency > 0.0 {
                        current_latency
                    } else {
                        result.latency
                    })
            };

            let is_failure = !current_success;

            // 判断是否严重变慢（基于当前绑定的 IP）
            let slow_ratio = if baseline > 0.0 && current_latency > 0.0 {
                (current_latency - baseline) / baseline * 100.0
            } else {
                0.0
            };
            let severe_degraded = !is_failure
                && baseline > 0.0
                && slow_ratio >= slow_threshold as f64
                && (current_latency - baseline) >= SEVERE_ABS_THRESHOLD_MS;

            // 更新失败计数
            let consecutive_failures = {
                let mut counts = failure_counts.lock().await;
                let count = counts.entry(endpoint.domain.clone()).or_insert(0);
                if is_failure {
                    *count += 1;
                } else {
                    *count = 0;
                }
                *count
            };

            let failure_window_count = {
                let mut windows = failure_windows.lock().await;
                let window = windows
                    .entry(endpoint.domain.clone())
                    .or_insert_with(VecDeque::new);
                window.push_back(is_failure);
                if window.len() > FAILURE_WINDOW_SIZE {
                    window.pop_front();
                }
                window.iter().filter(|v| **v).count()
            };

            let severe_window_count = {
                let mut windows = severe_windows.lock().await;
                let window = windows
                    .entry(endpoint.domain.clone())
                    .or_insert_with(VecDeque::new);
                window.push_back(severe_degraded);
                if window.len() > SEVERE_WINDOW_SIZE {
                    window.pop_front();
                }
                window.iter().filter(|v| **v).count()
            };

            let last_switch = {
                let times = last_switch_times.lock().await;
                times.get(&endpoint.domain).copied().unwrap_or(0)
            };
            let in_cooldown = now - last_switch < SWITCH_COOLDOWN_SECS;

            let should_switch_for_failure = consecutive_failures >= failure_threshold
                && failure_window_count >= FAILURE_WINDOW_THRESHOLD;

            let should_switch_for_degradation = severe_window_count >= SEVERE_WINDOW_THRESHOLD;
            let switch_triggered = should_switch_for_failure || should_switch_for_degradation;
            let should_switch_now = {
                let mut pending = pending_switch_since.lock().await;
                Self::should_switch_after_silent_window(
                    &endpoint.domain,
                    now,
                    switch_triggered,
                    in_cooldown,
                    &mut pending,
                )
            };

            if should_switch_now {
                needs_switch.push(endpoint.clone());
            }

            // 最优 IP 信息（来自测试结果）
            let (best_ip, best_latency) = if result.success {
                (Some(result.ip.clone()), result.latency)
            } else if current_success {
                (current_ip.clone(), current_latency)
            } else {
                (None, 0.0)
            };

            endpoints_health.push(EndpointHealth {
                domain: endpoint.domain.clone(),
                current_ip,
                best_ip,
                best_latency,
                latency: current_latency,
                baseline_latency: baseline,
                consecutive_failures,
                is_healthy: !is_failure && !severe_degraded,
                recommend_retest: severe_degraded && !should_switch_now,
            });
        }

        CheckResult {
            endpoints_health,
            needs_switch,
        }
    }

    /// 执行切换
    async fn perform_switch(
        endpoints: &[Endpoint],
        baselines: &Arc<Mutex<HashMap<String, f64>>>,
        last_switch_times: &Arc<Mutex<HashMap<String, i64>>>,
        _config_manager: &ConfigManager,
    ) -> SwitchResult {
        let tester = EndpointTester::new(vec![]);

        // 准备阶段：收集测试结果
        struct PendingSwitch {
            domain: String,
            old_ip: Option<String>,
            new_ip: String,
            new_latency: f64,
        }
        let mut pending_switches: Vec<PendingSwitch> = Vec::new();
        let mut bindings: Vec<HostsBinding> = Vec::new();

        let unique_endpoints = Self::dedupe_endpoints_by_domain(endpoints.to_vec());
        for endpoint in &unique_endpoints {
            // 重新测试找最优 IP
            let result = tester.test_endpoint(endpoint).await;

            if result.success {
                // 记录旧 IP（在写入前读取）
                let old_ip = hosts_ops::read_binding(&endpoint.domain);
                if old_ip.as_deref() == Some(result.ip.as_str()) {
                    // IP 未变化时跳过写入，避免无意义 flushdns 打断现有连接
                    continue;
                }

                // 添加绑定
                bindings.push(HostsBinding {
                    domain: endpoint.domain.clone(),
                    ip: result.ip.clone(),
                });

                pending_switches.push(PendingSwitch {
                    domain: endpoint.domain.clone(),
                    old_ip,
                    new_ip: result.ip,
                    new_latency: result.latency,
                });
            }
        }

        // 批量写入 - 只有写入成功才报告切换成功
        if bindings.is_empty() {
            return SwitchResult {
                switched_count: 0,
                switched: Vec::new(),
            };
        }

        match hosts_ops::write_bindings_batch(&bindings) {
            Ok(_) => {
                // 写入成功，刷新 DNS
                let _ = hosts_ops::flush_dns();

                // 更新基准延迟（只有写入成功才更新）
                {
                    let mut b = baselines.lock().await;
                    for ps in &pending_switches {
                        b.insert(ps.domain.clone(), ps.new_latency);
                    }
                }

                {
                    let mut times = last_switch_times.lock().await;
                    let now = current_timestamp();
                    for ps in &pending_switches {
                        times.insert(ps.domain.clone(), now);
                    }
                }

                // 构建成功结果
                let switched: Vec<SwitchedEndpoint> = pending_switches
                    .into_iter()
                    .map(|ps| SwitchedEndpoint {
                        domain: ps.domain,
                        old_ip: ps.old_ip,
                        new_ip: ps.new_ip,
                        new_latency: ps.new_latency,
                    })
                    .collect();

                SwitchResult {
                    switched_count: switched.len() as u32,
                    switched,
                }
            }
            Err(e) => {
                // 写入失败，记录错误并返回空结果
                eprintln!("Failed to write bindings: {}", e);
                SwitchResult {
                    switched_count: 0,
                    switched: Vec::new(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(name: &str, domain: &str) -> Endpoint {
        Endpoint {
            name: name.to_string(),
            url: format!("https://{}/health", domain),
            domain: domain.to_string(),
            enabled: true,
        }
    }

    #[test]
    fn dedupe_endpoints_by_domain_keeps_first_item() {
        let endpoints = vec![
            endpoint("a-1", "a.com"),
            endpoint("a-2", "a.com"),
            endpoint("b-1", "b.com"),
        ];
        let deduped = HealthChecker::dedupe_endpoints_by_domain(endpoints);

        assert_eq!(deduped.len(), 2);
        assert_eq!(deduped[0].name, "a-1");
        assert_eq!(deduped[1].name, "b-1");
    }

    #[test]
    fn defensive_thresholds_are_not_too_aggressive() {
        assert!(MIN_CHECK_INTERVAL_SECS >= 60);
        assert!(MIN_SLOW_THRESHOLD_PERCENT >= 100);
        assert!(MIN_FAILURE_THRESHOLD >= 3);
    }

    #[test]
    fn silent_window_defers_first_switch() {
        let mut pending = HashMap::new();
        let now = 1_000;
        let should_switch = HealthChecker::should_switch_after_silent_window(
            "a.com",
            now,
            true,
            false,
            &mut pending,
        );
        assert!(!should_switch);
        assert_eq!(pending.get("a.com"), Some(&now));
    }

    #[test]
    fn silent_window_switches_after_timeout() {
        let mut pending = HashMap::new();
        let _ = HealthChecker::should_switch_after_silent_window(
            "a.com",
            1_000,
            true,
            false,
            &mut pending,
        );
        let should_switch = HealthChecker::should_switch_after_silent_window(
            "a.com",
            1_000 + SWITCH_SILENT_WINDOW_SECS,
            true,
            false,
            &mut pending,
        );
        assert!(should_switch);
    }

    #[test]
    fn silent_window_clears_on_recovery() {
        let mut pending = HashMap::new();
        let _ = HealthChecker::should_switch_after_silent_window(
            "a.com",
            1_000,
            true,
            false,
            &mut pending,
        );
        let should_switch = HealthChecker::should_switch_after_silent_window(
            "a.com",
            1_010,
            false,
            false,
            &mut pending,
        );
        assert!(!should_switch);
        assert!(!pending.contains_key("a.com"));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub endpoints_health: Vec<EndpointHealth>,
    pub needs_switch: Vec<Endpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchResult {
    pub switched_count: u32,
    pub switched: Vec<SwitchedEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchedEndpoint {
    pub domain: String,
    pub old_ip: Option<String>,
    pub new_ip: String,
    pub new_latency: f64,
}
