//! Data models

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub name: String,
    pub url: String,
    pub domain: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointResult {
    pub endpoint: Endpoint,
    pub ip: String,
    pub latency: f64,
    pub ttfb: f64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    // 新增字段: 加速百分比显示 + 智能回退
    #[serde(default)]
    pub original_ip: String,
    #[serde(default)]
    pub original_latency: f64,
    #[serde(default)]
    pub speedup_percent: f64,
    #[serde(default)]
    pub use_original: bool,
}

impl EndpointResult {
    pub fn success(endpoint: Endpoint, ip: String, latency: f64) -> Self {
        Self {
            endpoint,
            ip,
            latency,
            ttfb: latency,
            success: true,
            error: None,
            original_ip: String::new(),
            original_latency: 0.0,
            speedup_percent: 0.0,
            use_original: false,
        }
    }

    pub fn success_with_comparison(
        endpoint: Endpoint,
        ip: String,
        latency: f64,
        original_ip: String,
        original_latency: f64,
    ) -> Self {
        let speedup_percent = if original_latency > 0.0 && latency < 9999.0 {
            (original_latency - latency) / original_latency * 100.0
        } else {
            0.0
        };
        let use_original = speedup_percent <= 0.0;

        Self {
            endpoint,
            ip: if use_original {
                original_ip.clone()
            } else {
                ip
            },
            latency: if use_original {
                original_latency
            } else {
                latency
            },
            ttfb: if use_original {
                original_latency
            } else {
                latency
            },
            success: true,
            error: None,
            original_ip,
            original_latency,
            speedup_percent,
            use_original,
        }
    }

    pub fn failure(endpoint: Endpoint, ip: String, error: String) -> Self {
        Self {
            endpoint,
            ip,
            latency: 9999.0,
            ttfb: 9999.0,
            success: false,
            error: Some(error),
            original_ip: String::new(),
            original_latency: 0.0,
            speedup_percent: 0.0,
            use_original: false,
        }
    }
}

// 历史记录模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub timestamp: i64,
    pub domain: String,
    pub original_latency: f64,
    pub optimized_latency: f64,
    pub speedup_percent: f64,
    pub applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryStats {
    pub total_tests: u32,
    pub total_speedup_ms: f64,
    pub avg_speedup_percent: f64,
    pub records: Vec<HistoryRecord>,
}

/// Permission status for hosts file operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionStatus {
    pub has_permission: bool,
    pub is_using_service: bool,
}

/// 更新信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_url: String,
    pub release_notes: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(default = "default_slow_threshold")]
    pub slow_threshold: u32,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "default_test_count")]
    pub test_count: u32,
    #[serde(default = "default_minimize")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: bool,
    #[serde(default = "default_clear_on_exit")]
    pub clear_on_exit: bool,
    #[serde(default)]
    pub cloudflare_ips: Vec<String>,
    #[serde(default = "default_endpoints")]
    pub endpoints: Vec<Endpoint>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            check_interval: default_check_interval(),
            slow_threshold: default_slow_threshold(),
            failure_threshold: default_failure_threshold(),
            test_count: default_test_count(),
            minimize_to_tray: default_minimize(),
            close_to_tray: default_close_to_tray(),
            clear_on_exit: default_clear_on_exit(),
            cloudflare_ips: Vec::new(),
            endpoints: default_endpoints(),
        }
    }
}

fn default_mode() -> String {
    "auto".into()
}
fn default_check_interval() -> u64 {
    30
} // 30秒检查间隔
fn default_slow_threshold() -> u32 {
    50
} // 比基准慢50%触发切换
fn default_failure_threshold() -> u32 {
    3
} // 连续3次失败触发切换
fn default_test_count() -> u32 {
    3
}
fn default_minimize() -> bool {
    true
}
fn default_close_to_tray() -> bool {
    true
} // 关闭按钮最小化到托盘
fn default_clear_on_exit() -> bool {
    false
} // 退出时清除 hosts 绑定（默认关闭）

fn default_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            name: "WZW 代理".into(),
            url: "https://wzw.pp.ua/v1".into(),
            domain: "wzw.pp.ua".into(),
            enabled: true,
        },
        Endpoint {
            name: "BetterClaude".into(),
            url: "https://betterclau.de/claude/anyrouter.top".into(),
            domain: "betterclau.de".into(),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_creation() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com/api".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        assert_eq!(ep.name, "Test");
        assert_eq!(ep.domain, "test.com");
        assert!(ep.enabled);
    }

    #[test]
    fn test_endpoint_result_success() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        let result = EndpointResult::success(ep.clone(), "1.2.3.4".into(), 100.0);

        assert!(result.success);
        assert_eq!(result.ip, "1.2.3.4");
        assert_eq!(result.latency, 100.0);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_endpoint_result_failure() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        let result = EndpointResult::failure(ep.clone(), "1.2.3.4".into(), "Timeout".into());

        assert!(!result.success);
        assert_eq!(result.latency, 9999.0);
        assert_eq!(result.error, Some("Timeout".into()));
    }

    #[test]
    fn test_endpoint_result_with_comparison_speedup() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        // Original: 200ms, Optimized: 100ms -> 50% speedup
        let result = EndpointResult::success_with_comparison(
            ep,
            "1.2.3.4".into(),
            100.0,
            "5.6.7.8".into(),
            200.0,
        );

        assert!(result.success);
        assert_eq!(result.ip, "1.2.3.4"); // Should use optimized IP
        assert_eq!(result.latency, 100.0);
        assert_eq!(result.original_ip, "5.6.7.8");
        assert_eq!(result.original_latency, 200.0);
        assert!((result.speedup_percent - 50.0).abs() < 0.1);
        assert!(!result.use_original);
    }

    #[test]
    fn test_endpoint_result_with_comparison_use_original() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        // Original: 100ms, Optimized: 150ms -> original is better
        let result = EndpointResult::success_with_comparison(
            ep,
            "1.2.3.4".into(),
            150.0,
            "5.6.7.8".into(),
            100.0,
        );

        assert!(result.success);
        assert_eq!(result.ip, "5.6.7.8"); // Should use original IP
        assert_eq!(result.latency, 100.0); // Should use original latency
        assert!(result.use_original);
    }

    #[test]
    fn test_endpoint_result_with_comparison_equal() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        // Original == Optimized -> use original
        let result = EndpointResult::success_with_comparison(
            ep,
            "1.2.3.4".into(),
            100.0,
            "5.6.7.8".into(),
            100.0,
        );

        assert!(result.use_original);
        assert_eq!(result.ip, "5.6.7.8");
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();

        assert_eq!(config.mode, "auto");
        assert_eq!(config.check_interval, 30);
        assert_eq!(config.slow_threshold, 50);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.test_count, 3);
        assert!(config.minimize_to_tray);
        assert!(config.close_to_tray);
        assert!(!config.clear_on_exit); // 默认关闭
        assert!(config.cloudflare_ips.is_empty());
        assert_eq!(config.endpoints.len(), 2);
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.mode, parsed.mode);
        assert_eq!(config.check_interval, parsed.check_interval);
        assert_eq!(config.endpoints.len(), parsed.endpoints.len());
    }

    #[test]
    fn test_history_record() {
        let record = HistoryRecord {
            timestamp: 1706400000,
            domain: "test.com".into(),
            original_latency: 200.0,
            optimized_latency: 100.0,
            speedup_percent: 50.0,
            applied: true,
        };

        assert_eq!(record.domain, "test.com");
        assert!(record.applied);
    }

    #[test]
    fn test_history_stats_default() {
        let stats = HistoryStats::default();

        assert_eq!(stats.total_tests, 0);
        assert_eq!(stats.total_speedup_ms, 0.0);
        assert_eq!(stats.avg_speedup_percent, 0.0);
        assert!(stats.records.is_empty());
    }
}
