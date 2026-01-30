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

/// 工作流执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResult {
    pub test_count: u32,
    pub success_count: u32,
    pub applied_count: u32,
    pub results: Vec<EndpointResult>,
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
        }
    }
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

fn default_autostart() -> bool {
    false
} // 开机自启动（默认关闭）

fn default_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            name: "anyrouter大善人".into(),
            url: "https://betterclau.de/claude/anyrouter.top".into(),
            domain: "betterclau.de".into(),
            enabled: true,
        },
        Endpoint {
            name: "L站WONG大佬".into(),
            url: "https://wzw.pp.ua/v1".into(),
            domain: "wzw.pp.ua".into(),
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

        assert_eq!(config.check_interval, 30);
        assert_eq!(config.slow_threshold, 50);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.test_count, 3);
        assert!(!config.autostart); // 默认关闭
        assert_eq!(config.endpoints.len(), 2);
    }

    #[test]
    fn test_app_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AppConfig = serde_json::from_str(&json).unwrap();

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

    #[test]
    fn test_workflow_result_creation() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        let endpoint_result = EndpointResult::success(ep, "1.2.3.4".into(), 100.0);

        let workflow_result = WorkflowResult {
            test_count: 2,
            success_count: 1,
            applied_count: 1,
            results: vec![endpoint_result],
        };

        assert_eq!(workflow_result.test_count, 2);
        assert_eq!(workflow_result.success_count, 1);
        assert_eq!(workflow_result.applied_count, 1);
        assert_eq!(workflow_result.results.len(), 1);
    }

    #[test]
    fn test_workflow_result_serialization() {
        let ep = Endpoint {
            name: "Test".into(),
            url: "https://test.com".into(),
            domain: "test.com".into(),
            enabled: true,
        };
        let endpoint_result = EndpointResult::success(ep, "1.2.3.4".into(), 100.0);

        let workflow_result = WorkflowResult {
            test_count: 2,
            success_count: 1,
            applied_count: 1,
            results: vec![endpoint_result],
        };

        let json = serde_json::to_string(&workflow_result).unwrap();
        // Verify camelCase serialization
        assert!(json.contains("testCount"));
        assert!(json.contains("successCount"));
        assert!(json.contains("appliedCount"));

        let parsed: WorkflowResult = serde_json::from_str(&json).unwrap();
        assert_eq!(workflow_result.test_count, parsed.test_count);
        assert_eq!(workflow_result.success_count, parsed.success_count);
        assert_eq!(workflow_result.applied_count, parsed.applied_count);
        assert_eq!(workflow_result.results.len(), parsed.results.len());
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
        let json = r#"{"mode":"auto","check_interval":30}"#;
        let parsed: AppConfig = serde_json::from_str(json).unwrap();
        assert!(!parsed.autostart);
    }
}
