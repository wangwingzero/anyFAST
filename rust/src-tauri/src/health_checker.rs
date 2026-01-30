//! 后台健康检查模块
//! 定期检测当前绑定的 hosts 是否正常工作

use crate::config::ConfigManager;
use crate::endpoint_tester::EndpointTester;
use crate::hosts_manager::HostsBinding;
use crate::hosts_ops;
use crate::models::{AppConfig, Endpoint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    /// 启动后台健康检查（在内部 spawn 任务，立即返回）
    pub fn start(&self, app_handle: AppHandle, config: AppConfig) {
        let cancel_token = self.cancel_token.clone();
        let status = self.status.clone();
        let baselines = self.baselines.clone();
        let config_manager = self.config_manager.clone();

        let check_interval = config.check_interval;
        let slow_threshold = config.slow_threshold;
        let failure_threshold = config.failure_threshold;

        // 获取启用的端点
        let endpoints: Vec<Endpoint> = config.endpoints.into_iter().filter(|e| e.enabled).collect();

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
        slow_threshold: u32,
        failure_threshold: u32,
    ) -> CheckResult {
        let tester = EndpointTester::new(vec![]);
        let mut endpoints_health = Vec::new();
        let mut needs_switch = Vec::new();

        for endpoint in endpoints {
            // 获取当前绑定的 IP
            let current_ip = hosts_ops::read_binding(&endpoint.domain);

            // 测试端点（会测试所有 Cloudflare IP 并返回最优结果）
            let result = tester.test_endpoint(endpoint).await;

            // 获取基准延迟
            let baseline = {
                let b = baselines.lock().await;
                b.get(&endpoint.domain).copied().unwrap_or(result.latency)
            };

            // 判断是否健康（基于当前绑定的 IP）
            let is_healthy = if result.success {
                // 检查是否比基准慢超过阈值
                let slow_ratio = if baseline > 0.0 {
                    (result.latency - baseline) / baseline * 100.0
                } else {
                    0.0
                };
                slow_ratio < slow_threshold as f64
            } else {
                false
            };

            // 更新失败计数
            {
                let mut counts = failure_counts.lock().await;
                let count = counts.entry(endpoint.domain.clone()).or_insert(0);
                if is_healthy {
                    *count = 0;
                } else {
                    *count += 1;
                }

                // 如果连续失败次数达到阈值，需要切换
                if *count >= failure_threshold {
                    needs_switch.push(endpoint.clone());
                    *count = 0; // 重置计数
                }
            }

            // 最优 IP 信息（来自测试结果）
            let (best_ip, best_latency) = if result.success {
                (Some(result.ip.clone()), result.latency)
            } else {
                (None, 0.0)
            };

            // 智能切换：如果最优 IP 与当前绑定不同，且延迟改善超过 20%，触发切换
            if let Some(current) = current_ip.as_ref() {
                if result.success && current != &result.ip {
                    // 计算改善百分比（相对于基准延迟）
                    let improvement = if baseline > 0.0 {
                        (baseline - result.latency) / baseline * 100.0
                    } else {
                        0.0
                    };
                    // 如果改善超过 20%，触发切换
                    if improvement > 20.0
                        && !needs_switch.iter().any(|e| e.domain == endpoint.domain)
                    {
                        needs_switch.push(endpoint.clone());
                    }
                }
            }

            endpoints_health.push(EndpointHealth {
                domain: endpoint.domain.clone(),
                current_ip,
                best_ip,
                best_latency,
                latency: result.latency,
                baseline_latency: baseline,
                consecutive_failures: {
                    let counts = failure_counts.lock().await;
                    *counts.get(&endpoint.domain).unwrap_or(&0)
                },
                is_healthy,
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

        for endpoint in endpoints {
            // 重新测试找最优 IP
            let result = tester.test_endpoint(endpoint).await;

            if result.success {
                // 记录旧 IP（在写入前读取）
                let old_ip = hosts_ops::read_binding(&endpoint.domain);

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
