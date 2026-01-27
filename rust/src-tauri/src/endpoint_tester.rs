//! Endpoint speed tester with Cloudflare IP optimization

use crate::models::{Endpoint, EndpointResult};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;

/// 日志宏：输出带时间戳的调试日志到 stderr
macro_rules! debug_log {
    ($($arg:tt)*) => {
        eprintln!("[{}] [DEBUG] {}", chrono::Local::now().format("%H:%M:%S%.3f"), format!($($arg)*));
    };
}

macro_rules! info_log {
    ($($arg:tt)*) => {
        eprintln!("[{}] [INFO] {}", chrono::Local::now().format("%H:%M:%S%.3f"), format!($($arg)*));
    };
}

macro_rules! warn_log {
    ($($arg:tt)*) => {
        eprintln!("[{}] [WARN] {}", chrono::Local::now().format("%H:%M:%S%.3f"), format!($($arg)*));
    };
}

macro_rules! error_log {
    ($($arg:tt)*) => {
        eprintln!("[{}] [ERROR] {}", chrono::Local::now().format("%H:%M:%S%.3f"), format!($($arg)*));
    };
}

/// Default Cloudflare IPs for optimization
const DEFAULT_CF_IPS: &[&str] = &[
    "104.16.0.1", "104.17.0.1", "104.18.0.1", "104.19.0.1",
    "104.20.0.1", "104.21.0.1", "104.22.0.1", "104.23.0.1",
    "172.67.0.1", "172.67.100.1", "162.159.0.1",
];

/// Cloudflare IP ranges for detection
const CF_RANGES: &[&str] = &[
    "104.16.", "104.17.", "104.18.", "104.19.",
    "104.20.", "104.21.", "104.22.", "104.23.",
    "104.24.", "104.25.", "104.26.", "104.27.",
    "172.67.", "162.159.",
];

/// Check if an IP is in Cloudflare's range
pub fn is_cloudflare_ip(ip: &str) -> bool {
    CF_RANGES.iter().any(|r| ip.starts_with(r))
}

/// Get Cloudflare IPs to test (custom or default)
pub fn get_cf_ips(custom_cf_ips: &[String]) -> Vec<String> {
    if !custom_cf_ips.is_empty() {
        custom_cf_ips.to_vec()
    } else {
        DEFAULT_CF_IPS.iter().map(|s| s.to_string()).collect()
    }
}

/// Reusable endpoint tester with connection pooling
#[derive(Clone)]
pub struct EndpointTester {
    custom_cf_ips: Arc<Vec<String>>,
    cancelled: Arc<AtomicBool>,
    resolver: Arc<TokioAsyncResolver>,
    tls_connector: TlsConnector,
}

impl EndpointTester {
    pub fn new(custom_cf_ips: Vec<String>) -> Self {
        // Pre-create TLS configuration (reused across all connections)
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let tls_connector = TlsConnector::from(Arc::new(config));

        // Pre-create DNS resolver with caching
        let mut opts = ResolverOpts::default();
        opts.cache_size = 128;
        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), opts);

        Self {
            custom_cf_ips: Arc::new(custom_cf_ips),
            cancelled: Arc::new(AtomicBool::new(false)),
            resolver: Arc::new(resolver),
            tls_connector,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    fn get_cf_ips(&self) -> Vec<String> {
        get_cf_ips(&self.custom_cf_ips)
    }

    /// Test all endpoints concurrently with controlled parallelism
    pub async fn test_all(&self, endpoints: &[Endpoint]) -> Vec<EndpointResult> {
        info_log!("开始测试 {} 个端点", endpoints.len());

        if endpoints.is_empty() {
            warn_log!("端点列表为空，直接返回");
            return Vec::new();
        }

        // Limit concurrent endpoint tests to avoid overwhelming the system
        let max_concurrency = endpoints.len().min(8);
        debug_log!("最大并发数: {}", max_concurrency);
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut join_set = JoinSet::new();

        for (idx, endpoint) in endpoints.iter().enumerate() {
            if self.cancelled.load(Ordering::SeqCst) {
                warn_log!("测试已取消，停止添加新任务");
                break;
            }

            debug_log!("[{}/{}] 准备测试端点: {} ({})", idx + 1, endpoints.len(), endpoint.name, endpoint.domain);

            let endpoint = endpoint.clone();
            let tester = self.clone();

            // 获取信号量，添加 5 秒超时防止死锁
            let permit = match tokio::time::timeout(
                Duration::from_secs(5),
                semaphore.clone().acquire_owned()
            ).await {
                Ok(Ok(permit)) => permit,
                Ok(Err(_)) => {
                    error_log!("信号量关闭，停止测试");
                    break;
                }
                Err(_) => {
                    error_log!("获取信号量超时，跳过端点: {}", endpoint.name);
                    continue;
                }
            };

            let idx_copy = idx;
            let total = endpoints.len();
            join_set.spawn(async move {
                let _permit = permit;
                debug_log!("[{}/{}] 开始测试: {}", idx_copy + 1, total, endpoint.name);
                let start = Instant::now();
                let result = tester.test_endpoint(&endpoint).await;
                debug_log!("[{}/{}] 测试完成: {} - {} (耗时 {:.1}s)",
                    idx_copy + 1, total, endpoint.name,
                    if result.success { format!("成功 {:.0}ms", result.latency) } else { format!("失败: {}", result.error.as_deref().unwrap_or("unknown")) },
                    start.elapsed().as_secs_f64()
                );
                result
            });
        }

        info_log!("已添加 {} 个测试任务，等待完成...", join_set.len());

        let mut results = Vec::with_capacity(endpoints.len());
        let collect_start = Instant::now();

        // 收集结果，添加总体超时保护（30秒）
        loop {
            // 检查总体超时
            if collect_start.elapsed() > Duration::from_secs(30) {
                warn_log!("收集结果超时（30秒），已收集 {} 个结果，中止剩余任务", results.len());
                join_set.abort_all();
                break;
            }

            // 检查是否取消
            if self.cancelled.load(Ordering::SeqCst) {
                warn_log!("测试已取消，中止所有任务");
                join_set.abort_all();
                break;
            }

            // 使用超时等待下一个结果
            match tokio::time::timeout(Duration::from_secs(5), join_set.join_next()).await {
                Ok(Some(Ok(result))) => {
                    results.push(result);
                    debug_log!("已收集 {}/{} 个结果", results.len(), endpoints.len());
                }
                Ok(Some(Err(e))) => {
                    error_log!("任务 panic: {:?}", e);
                }
                Ok(None) => {
                    // 所有任务完成
                    info_log!("所有任务完成，共 {} 个结果", results.len());
                    break;
                }
                Err(_) => {
                    // 单个等待超时，继续等待
                    debug_log!("等待下一个结果超时，继续...");
                }
            }
        }

        // Sort by latency
        results.sort_by(|a, b| a.latency.partial_cmp(&b.latency).unwrap());

        let success_count = results.iter().filter(|r| r.success).count();
        info_log!("测速完成: {}/{} 成功, 最佳延迟: {:.0}ms",
            success_count,
            results.len(),
            results.first().map(|r| r.latency).unwrap_or(0.0)
        );

        results
    }

    /// Test a single endpoint and find the best IP
    pub async fn test_endpoint(&self, endpoint: &Endpoint) -> EndpointResult {
        debug_log!("test_endpoint 开始: {} ({})", endpoint.name, endpoint.domain);

        if self.cancelled.load(Ordering::SeqCst) {
            warn_log!("test_endpoint: 检测到取消信号");
            return EndpointResult::failure(
                endpoint.clone(),
                String::new(),
                "已取消".into(),
            );
        }

        // Resolve DNS using cached resolver (with 10s timeout)
        debug_log!("  DNS 解析: {}", endpoint.domain);
        let dns_start = Instant::now();
        let dns_result = tokio::time::timeout(
            Duration::from_secs(10),
            self.resolver.lookup_ip(&endpoint.domain)
        ).await;

        let dns_ips: Vec<String> = match dns_result {
            Ok(Ok(lookup)) => {
                let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                debug_log!("  DNS 成功 ({:.1}ms): {} 个 IP - {:?}",
                    dns_start.elapsed().as_secs_f64() * 1000.0, ips.len(), ips);
                ips
            }
            Ok(Err(e)) => {
                error_log!("  DNS 失败: {}", e);
                return EndpointResult::failure(
                    endpoint.clone(),
                    String::new(),
                    format!("DNS失败: {}", e),
                );
            }
            Err(_) => {
                error_log!("  DNS 超时 (10s)");
                return EndpointResult::failure(
                    endpoint.clone(),
                    String::new(),
                    "DNS超时".into(),
                );
            }
        };

        if dns_ips.is_empty() {
            error_log!("  DNS 无结果");
            return EndpointResult::failure(endpoint.clone(), String::new(), "DNS无结果".into());
        }

        // 记录原始 IP（DNS 解析的第一个 IP）
        let original_ip = dns_ips[0].clone();
        debug_log!("  原始 IP: {}", original_ip);

        // 先测试原始 IP 的延迟
        debug_log!("  测试原始 IP: {}", original_ip);
        let original_result = self.test_single_ip(endpoint, original_ip.clone()).await;
        let original_latency = if original_result.success {
            debug_log!("  原始 IP 延迟: {:.0}ms", original_result.latency);
            original_result.latency
        } else {
            debug_log!("  原始 IP 失败: {}", original_result.error.as_deref().unwrap_or("unknown"));
            9999.0
        };

        // Check if Cloudflare
        let is_cf = dns_ips.iter().any(|ip| is_cloudflare_ip(ip));
        if is_cf {
            debug_log!("  检测到 Cloudflare IP，启用 CF 优选");
        }

        // Collect IPs to test
        let test_ips: Vec<String> = if is_cf {
            let mut ips = self.get_cf_ips();
            ips.extend(dns_ips.clone());
            ips.into_iter().collect::<std::collections::HashSet<_>>().into_iter().take(15).collect()
        } else {
            dns_ips.clone()
        };

        debug_log!("  准备测试 {} 个 IP", test_ips.len());

        // Test all IPs concurrently with timeout
        let mut join_set = JoinSet::new();
        for ip in test_ips.iter() {
            let ep = endpoint.clone();
            let tester = self.clone();
            let ip_clone = ip.clone();
            join_set.spawn(async move {
                tester.test_single_ip(&ep, ip_clone).await
            });
        }

        // Collect results with 15s total timeout for all IP tests
        let mut best_result: Option<EndpointResult> = None;
        let ip_test_start = Instant::now();
        let ip_test_timeout = Duration::from_secs(15);

        loop {
            // 检查总超时
            if ip_test_start.elapsed() > ip_test_timeout {
                warn_log!("  IP 测试超时 (15s)，已测试部分 IP");
                join_set.abort_all();
                break;
            }

            // 检查取消
            if self.cancelled.load(Ordering::SeqCst) {
                warn_log!("  检测到取消信号，中止 IP 测试");
                join_set.abort_all();
                break;
            }

            // 等待下一个结果（3秒超时）
            match tokio::time::timeout(Duration::from_secs(3), join_set.join_next()).await {
                Ok(Some(Ok(result))) => {
                    if result.success {
                        if best_result.is_none() || result.latency < best_result.as_ref().unwrap().latency {
                            debug_log!("    IP {} 延迟 {:.0}ms (新最优)", result.ip, result.latency);
                            best_result = Some(result);
                        } else {
                            debug_log!("    IP {} 延迟 {:.0}ms", result.ip, result.latency);
                        }
                    } else {
                        debug_log!("    IP {} 失败: {}", result.ip, result.error.as_deref().unwrap_or("unknown"));
                    }
                }
                Ok(Some(Err(e))) => {
                    error_log!("    IP 测试任务 panic: {:?}", e);
                }
                Ok(None) => {
                    // 所有任务完成
                    debug_log!("  所有 IP 测试完成");
                    break;
                }
                Err(_) => {
                    // 继续等待
                    debug_log!("    等待 IP 测试结果...");
                }
            }
        }

        // 使用带比较功能的构造函数创建最终结果
        let final_result = if let Some(best) = best_result {
            info_log!("  端点 {} 最优 IP: {} ({:.0}ms, 原 {:.0}ms)",
                endpoint.name, best.ip, best.latency, original_latency);
            EndpointResult::success_with_comparison(
                endpoint.clone(),
                best.ip,
                best.latency,
                original_ip,
                original_latency,
            )
        } else if original_result.success {
            // 如果优化 IP 都失败，但原始 IP 成功，使用原始 IP
            info_log!("  端点 {} 使用原始 IP: {} ({:.0}ms)",
                endpoint.name, original_result.ip, original_result.latency);
            EndpointResult::success_with_comparison(
                endpoint.clone(),
                original_result.ip.clone(),
                original_result.latency,
                original_ip,
                original_latency,
            )
        } else {
            error_log!("  端点 {} 全部失败", endpoint.name);
            EndpointResult::failure(endpoint.clone(), original_ip, "全部超时".into())
        };

        debug_log!("test_endpoint 完成: {}", endpoint.name);
        final_result
    }

    async fn test_single_ip(&self, endpoint: &Endpoint, ip: String) -> EndpointResult {
        let timeout = Duration::from_secs(5);

        match tokio::time::timeout(timeout, self.do_https_test(endpoint, &ip)).await {
            Ok(Ok(latency)) => EndpointResult::success(endpoint.clone(), ip, latency),
            Ok(Err(e)) => EndpointResult::failure(endpoint.clone(), ip, e),
            Err(_) => EndpointResult::failure(endpoint.clone(), ip, "超时".into()),
        }
    }

    async fn do_https_test(&self, endpoint: &Endpoint, ip: &str) -> Result<f64, String> {
        let addr: SocketAddr = format!("{}:443", ip)
            .parse()
            .map_err(|e| format!("Invalid IP: {}", e))?;

        let start = Instant::now();

        // TCP connect
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("TCP: {}", e))?;

        // TLS handshake using reusable connector
        let connector = self.tls_connector.clone();
        let domain = endpoint.domain.clone().try_into().map_err(|_| "Invalid domain")?;

        let mut tls_stream = connector
            .connect(domain, stream)
            .await
            .map_err(|e| format!("TLS: {}", e))?;

        // Extract path from URL - properly parse to avoid matching scheme slashes
        let path = {
            let url_str = endpoint.url.as_str();
            // Find the path after the domain
            if let Some(scheme_end) = url_str.find("://") {
                let after_scheme = &url_str[scheme_end + 3..];
                if let Some(path_start) = after_scheme.find('/') {
                    &after_scheme[path_start..]
                } else {
                    "/"
                }
            } else if url_str.starts_with('/') {
                url_str
            } else {
                "/"
            }
        };

        // Send HTTP HEAD request
        let request = format!(
            "HEAD {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: AnyRouter/1.0\r\nConnection: close\r\n\r\n",
            path, endpoint.domain
        );

        tls_stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| format!("Write: {}", e))?;

        // Read response header
        let mut buf = [0u8; 1024];
        let n = tls_stream
            .read(&mut buf)
            .await
            .map_err(|e| format!("Read: {}", e))?;

        let latency = start.elapsed().as_secs_f64() * 1000.0;

        // Verify HTTP response
        let response = String::from_utf8_lossy(&buf[..n]);
        if response.starts_with("HTTP/") {
            Ok(latency)
        } else {
            Err("Invalid response".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cloudflare_ip_104_16() {
        assert!(is_cloudflare_ip("104.16.0.1"));
        assert!(is_cloudflare_ip("104.16.123.45"));
    }

    #[test]
    fn test_is_cloudflare_ip_104_17_to_27() {
        assert!(is_cloudflare_ip("104.17.0.1"));
        assert!(is_cloudflare_ip("104.18.0.1"));
        assert!(is_cloudflare_ip("104.19.0.1"));
        assert!(is_cloudflare_ip("104.20.0.1"));
        assert!(is_cloudflare_ip("104.21.0.1"));
        assert!(is_cloudflare_ip("104.22.0.1"));
        assert!(is_cloudflare_ip("104.23.0.1"));
        assert!(is_cloudflare_ip("104.24.0.1"));
        assert!(is_cloudflare_ip("104.25.0.1"));
        assert!(is_cloudflare_ip("104.26.0.1"));
        assert!(is_cloudflare_ip("104.27.0.1"));
    }

    #[test]
    fn test_is_cloudflare_ip_172_67() {
        assert!(is_cloudflare_ip("172.67.0.1"));
        assert!(is_cloudflare_ip("172.67.100.200"));
    }

    #[test]
    fn test_is_cloudflare_ip_162_159() {
        assert!(is_cloudflare_ip("162.159.0.1"));
        assert!(is_cloudflare_ip("162.159.128.100"));
    }

    #[test]
    fn test_is_cloudflare_ip_not_cf() {
        assert!(!is_cloudflare_ip("1.1.1.1"));
        assert!(!is_cloudflare_ip("8.8.8.8"));
        assert!(!is_cloudflare_ip("192.168.1.1"));
        assert!(!is_cloudflare_ip("10.0.0.1"));
        assert!(!is_cloudflare_ip("104.15.0.1"));  // Close but not CF
        assert!(!is_cloudflare_ip("104.28.0.1"));  // Close but not CF
    }

    #[test]
    fn test_get_cf_ips_default() {
        let custom: Vec<String> = vec![];
        let ips = get_cf_ips(&custom);

        assert_eq!(ips.len(), 11);
        assert!(ips.contains(&"104.16.0.1".to_string()));
        assert!(ips.contains(&"172.67.0.1".to_string()));
        assert!(ips.contains(&"162.159.0.1".to_string()));
    }

    #[test]
    fn test_get_cf_ips_custom() {
        let custom = vec!["1.2.3.4".to_string(), "5.6.7.8".to_string()];
        let ips = get_cf_ips(&custom);

        assert_eq!(ips.len(), 2);
        assert_eq!(ips[0], "1.2.3.4");
        assert_eq!(ips[1], "5.6.7.8");
    }

    #[test]
    fn test_get_cf_ips_custom_overrides_default() {
        let custom = vec!["custom.ip.1.1".to_string()];
        let ips = get_cf_ips(&custom);

        assert_eq!(ips.len(), 1);
        assert!(!ips.contains(&"104.16.0.1".to_string()));
    }

    #[test]
    fn test_default_cf_ips_are_valid() {
        for ip in DEFAULT_CF_IPS {
            // All default CF IPs should be recognized as CF IPs
            assert!(is_cloudflare_ip(ip), "IP {} should be recognized as CF", ip);
        }
    }

    #[test]
    fn test_cf_ranges_coverage() {
        // Verify that CF_RANGES covers expected prefixes
        assert!(CF_RANGES.contains(&"104.16."));
        assert!(CF_RANGES.contains(&"104.27."));
        assert!(CF_RANGES.contains(&"172.67."));
        assert!(CF_RANGES.contains(&"162.159."));

        // Should have 14 ranges total
        assert_eq!(CF_RANGES.len(), 14);
    }

    #[tokio::test]
    async fn test_endpoint_tester_creation() {
        let tester = EndpointTester::new(vec![]);

        // Verify it can be cloned (required for concurrent testing)
        let _cloned = tester.clone();
    }

    #[tokio::test]
    async fn test_endpoint_tester_with_custom_ips() {
        let custom_ips = vec!["1.2.3.4".to_string()];
        let tester = EndpointTester::new(custom_ips);

        let ips = tester.get_cf_ips();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0], "1.2.3.4");
    }

    #[tokio::test]
    async fn test_endpoint_tester_cancel() {
        let tester = EndpointTester::new(vec![]);

        // Initially not cancelled
        assert!(!tester.cancelled.load(Ordering::SeqCst));

        // After cancel
        tester.cancel();
        assert!(tester.cancelled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_test_all_empty_endpoints() {
        let tester = EndpointTester::new(vec![]);
        let results = tester.test_all(&[]).await;

        assert!(results.is_empty());
    }
}
