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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
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
            warning: None,
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
        // 计算加速百分比（始终和原始 DNS IP 对比）
        let speedup_percent = if original_latency > 0.0 && latency < 9999.0 {
            (original_latency - latency) / original_latency * 100.0
        } else {
            0.0
        };

        // 始终使用测试中最快的 IP，不回退到原始 IP
        // use_original 仅用于标记当前使用的 IP 是否恰好是原始 IP
        let use_original = ip == original_ip;

        Self {
            endpoint,
            ip,      // 始终使用传入的最优 IP
            latency, // 始终使用传入的最优延迟
            ttfb: latency,
            success: true,
            error: None,
            warning: None,
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
            warning: None,
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
    #[serde(default = "default_check_interval")]
    pub check_interval: u64,
    #[serde(default = "default_slow_threshold")]
    pub slow_threshold: u32,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "default_test_count")]
    pub test_count: u32,
    #[serde(default = "default_autostart")]
    pub autostart: bool,
    #[serde(default = "default_endpoints")]
    pub endpoints: Vec<Endpoint>,
    #[serde(default = "default_preferred_ips")]
    pub preferred_ips: Vec<String>,
    #[serde(default = "default_continuous_mode")]
    pub continuous_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            check_interval: default_check_interval(),
            slow_threshold: default_slow_threshold(),
            failure_threshold: default_failure_threshold(),
            test_count: default_test_count(),
            autostart: default_autostart(),
            endpoints: default_endpoints(),
            preferred_ips: default_preferred_ips(),
            continuous_mode: default_continuous_mode(),
        }
    }
}

fn default_check_interval() -> u64 {
    120
} // 120秒检查间隔
fn default_slow_threshold() -> u32 {
    150
} // 比基准慢150%且绝对增加300ms判定为严重变慢
fn default_failure_threshold() -> u32 {
    5
} // 连续5次失败触发切换
fn default_test_count() -> u32 {
    3
}

fn default_autostart() -> bool {
    false
} // 开机自启动（默认关闭）

fn default_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            name: "anyrouter".into(),
            url: "https://cf.betterclau.de/claude/anyrouter.top".into(),
            domain: "cf.betterclau.de".into(),
            enabled: true,
        },
        Endpoint {
            name: "WONG公益站".into(),
            url: "https://wzw.pp.ua".into(),
            domain: "wzw.pp.ua".into(),
            enabled: true,
        },
    ]
}

fn default_preferred_ips() -> Vec<String> {
    Vec::new()
}

fn default_continuous_mode() -> bool {
    true
}

/// 测速进度事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestProgressEventType {
    TestStarted,
    DnsResolved,
    DnsFailed,
    OriginalIpTested,
    CandidateTestComplete,
    CfThrottleDetected,
    NetworkUnreachable,
    TcpProbeStarted,
    TcpProbeComplete,
    EndpointComplete,
    TestFinished,
}

/// 测速进度事件（后端 → 前端实时日志）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestProgressEvent {
    pub event_type: TestProgressEventType,
    pub level: String,
    pub endpoint_name: Option<String>,
    pub message: String,
}

/// 持续优化事件类型
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationEventType {
    AutoSwitch,
    CheckComplete,
    Started,
    #[default]
    Stopped,
}

/// 持续优化事件（后端 → 前端通知）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationEvent {
    pub event_type: OptimizationEventType,
    pub domain: Option<String>,
    pub old_ip: Option<String>,
    pub new_ip: Option<String>,
    pub old_latency: Option<f64>,
    pub new_latency: Option<f64>,
    pub message: String,
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
        // 新逻辑：传入的 IP 就是最优 IP（调用方已经选好了）
        // 这里模拟原始 IP 就是最优的情况
        let result = EndpointResult::success_with_comparison(
            ep,
            "5.6.7.8".into(), // 传入原始 IP 作为最优
            100.0,
            "5.6.7.8".into(),
            100.0,
        );

        assert!(result.success);
        assert_eq!(result.ip, "5.6.7.8"); // 使用传入的 IP
        assert_eq!(result.latency, 100.0);
        assert!(result.use_original); // IP 等于原始 IP
    }

    #[test]
    fn test_endpoint_result_with_comparison_equal() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        // 传入的 IP 恰好等于原始 IP
        let result = EndpointResult::success_with_comparison(
            ep,
            "5.6.7.8".into(),
            100.0,
            "5.6.7.8".into(),
            100.0,
        );

        assert!(result.use_original); // IP 等于原始 IP
        assert_eq!(result.ip, "5.6.7.8");
        assert_eq!(result.speedup_percent, 0.0); // 没有加速
    }

    #[test]
    fn test_app_config_default() {
        let config = AppConfig::default();

        assert_eq!(config.check_interval, 120);
        assert_eq!(config.slow_threshold, 150);
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.test_count, 3);
        assert!(!config.autostart); // 默认关闭
        assert_eq!(config.endpoints.len(), 2); // 2个默认站点
        assert!(config.preferred_ips.is_empty()); // 默认自动优选
        assert!(config.continuous_mode); // 默认开启持续优化
        assert_eq!(config.endpoints[0].name, "anyrouter");
        assert!(config.endpoints[0].enabled);
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.check_interval, parsed.check_interval);
        assert_eq!(config.endpoints.len(), parsed.endpoints.len());
        assert_eq!(config.preferred_ips, parsed.preferred_ips);
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

    #[test]
    fn test_app_config_autostart_serialization() {
        // Test with autostart = true
        let config = AppConfig {
            autostart: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.autostart);

        // Test with autostart = false (default)
        let config2 = AppConfig::default();
        let json2 = serde_json::to_string(&config2).unwrap();
        let parsed2: AppConfig = serde_json::from_str(&json2).unwrap();
        assert!(!parsed2.autostart);
    }

    #[test]
    fn test_app_config_autostart_default_deserialization() {
        // Test that missing autostart field defaults to false
        let json = r#"{"mode":"auto","check_interval":120}"#;
        let parsed: AppConfig = serde_json::from_str(json).unwrap();
        assert!(!parsed.autostart);
        assert!(parsed.preferred_ips.is_empty());
        assert!(parsed.continuous_mode); // 缺失字段默认 true
    }
}
