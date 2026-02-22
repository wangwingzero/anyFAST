//! Endpoint speed tester with Cloudflare IP optimization

use crate::models::{Endpoint, EndpointResult};
use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use reqwest::Client;
use std::collections::HashSet;
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

/// Default Cloudflare IPs for optimization (fallback when online API fails)
const DEFAULT_CF_IPS: &[&str] = &[
    "104.16.0.1",
    "104.17.0.1",
    "104.18.0.1",
    "104.19.0.1",
    "104.20.0.1",
    "104.21.0.1",
    "104.22.0.1",
    "104.23.0.1",
    "172.67.0.1",
    "172.67.100.1",
    "162.159.0.1",
];

/// Online API for fetching optimized Cloudflare IPs (cf-speed-dns project)
const IPDB_API_URL: &str = "https://ip.164746.xyz/ipTop10.html";

/// Max number of candidate IPs for each endpoint
const MAX_TEST_IPS: usize = 15;
/// Max concurrent endpoint tests
const MAX_ENDPOINT_CONCURRENCY: usize = 6;
/// Max concurrent IP tests within one endpoint
const MAX_IP_CONCURRENCY_PER_ENDPOINT: usize = 6;
/// DNS lookup timeout for each endpoint
const DNS_LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);
/// Timeout for a single IP test
const SINGLE_IP_TEST_TIMEOUT: Duration = Duration::from_secs(8);
/// Total timeout for all IP tests within one endpoint
const IP_TEST_TOTAL_TIMEOUT: Duration = Duration::from_secs(45);
/// End-to-end workflow timeout bounds (used for dynamic estimation)
const MIN_WORKFLOW_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_WORKFLOW_TIMEOUT: Duration = Duration::from_secs(180);
/// Reserve some headroom for outer workflow timeout
const COLLECT_TIMEOUT_HEADROOM: Duration = Duration::from_secs(5);

/// Estimate a realistic timeout budget for testing `endpoint_count` endpoints.
/// This prevents long endpoint lists from starving later rows and being marked as 9999ms early.
pub fn estimate_test_timeout(endpoint_count: usize) -> Duration {
    if endpoint_count == 0 {
        return MIN_WORKFLOW_TIMEOUT;
    }

    let concurrency = endpoint_count.clamp(1, MAX_ENDPOINT_CONCURRENCY);
    let batches = endpoint_count.div_ceil(concurrency) as u64;

    // Worst-case per endpoint phase:
    // DNS lookup + original IP probe + optimized IP candidate probing.
    let per_endpoint_budget = DNS_LOOKUP_TIMEOUT.as_secs()
        + SINGLE_IP_TEST_TIMEOUT.as_secs()
        + IP_TEST_TOTAL_TIMEOUT.as_secs();

    // Add fixed scheduling overhead to avoid premature timeout in loaded environments.
    let estimated_secs = batches * per_endpoint_budget + 15;

    Duration::from_secs(estimated_secs.clamp(
        MIN_WORKFLOW_TIMEOUT.as_secs(),
        MAX_WORKFLOW_TIMEOUT.as_secs(),
    ))
}

/// 公共 DNS 解析器列表（用于非 CF 站点的多 DNS 优选）
const PUBLIC_DNS_SERVERS: &[&str] = &[
    "8.8.8.8",        // Google
    "8.8.4.4",        // Google
    "1.1.1.1",        // Cloudflare
    "1.0.0.1",        // Cloudflare
    "9.9.9.9",        // Quad9
    "208.67.222.222", // OpenDNS
    "223.5.5.5",      // AliDNS
    "223.6.6.6",      // AliDNS
];

/// 多 DNS 查询总超时
const MULTI_DNS_TIMEOUT: Duration = Duration::from_secs(3);

/// Cloudflare IP ranges for detection
const CF_RANGES: &[&str] = &[
    "104.16.", "104.17.", "104.18.", "104.19.", "104.20.", "104.21.", "104.22.", "104.23.",
    "104.24.", "104.25.", "104.26.", "104.27.", "172.67.", "162.159.",
];

/// Check if an IP is in Cloudflare's range
pub fn is_cloudflare_ip(ip: &str) -> bool {
    CF_RANGES.iter().any(|r| ip.starts_with(r))
}

/// Merge candidate IPs in stable order and deduplicate.
/// Priority: online CF IP list first, then current DNS IPs.
fn merge_candidate_ips(cf_ips: Vec<String>, dns_ips: &[String], limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut merged = Vec::with_capacity(limit);

    for ip in cf_ips.into_iter().chain(dns_ips.iter().cloned()) {
        if seen.insert(ip.clone()) {
            merged.push(ip);
            if merged.len() >= limit {
                break;
            }
        }
    }

    merged
}

/// 并发查询多个公共 DNS 解析器，收集域名的所有唯一 IP
async fn resolve_via_multi_dns(domain: &str) -> Vec<String> {
    let mut join_set = JoinSet::new();

    for &dns_server in PUBLIC_DNS_SERVERS {
        let domain = domain.to_string();
        let addr: std::net::IpAddr = dns_server.parse().unwrap();
        join_set.spawn(async move {
            let ns = NameServerConfig::new(SocketAddr::new(addr, 53), Protocol::Udp);
            let config = ResolverConfig::from_parts(None, vec![], vec![ns]);
            let mut opts = ResolverOpts::default();
            opts.timeout = Duration::from_secs(2);
            opts.attempts = 1;
            let resolver = TokioAsyncResolver::tokio(config, opts);
            match resolver.lookup_ip(&domain).await {
                Ok(lookup) => lookup.iter().map(|ip| ip.to_string()).collect::<Vec<_>>(),
                Err(_) => vec![],
            }
        });
    }

    // 收集结果，总超时 3 秒
    let mut all_ips = Vec::new();
    let start = Instant::now();
    while let Ok(Some(result)) = tokio::time::timeout(
        MULTI_DNS_TIMEOUT.saturating_sub(start.elapsed()),
        join_set.join_next(),
    )
    .await
    {
        if let Ok(ips) = result {
            all_ips.extend(ips);
        }
    }

    // 去重（保持顺序）
    let mut seen = HashSet::new();
    all_ips.retain(|ip| seen.insert(ip.clone()));
    all_ips
}

/// Fetch optimized Cloudflare IPs from online API
/// Returns IPs from cf-speed-dns, falls back to default IPs on failure
pub async fn fetch_online_cf_ips() -> Vec<String> {
    info_log!("从在线 API 获取优选 IP...");

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            warn_log!("创建 HTTP 客户端失败: {}, 使用默认 IP", e);
            return DEFAULT_CF_IPS.iter().map(|s| s.to_string()).collect();
        }
    };

    match client.get(IPDB_API_URL).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.text().await {
                    Ok(text) => {
                        // Support both comma-separated and newline-separated formats
                        let ips: Vec<String> = text
                            .split([',', '\n', '\r'])
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty() && !s.starts_with('#'))
                            .collect();

                        if ips.is_empty() {
                            warn_log!("在线 API 返回空列表，使用默认 IP");
                            DEFAULT_CF_IPS.iter().map(|s| s.to_string()).collect()
                        } else {
                            info_log!("从在线 API 获取到 {} 个优选 IP", ips.len());
                            ips
                        }
                    }
                    Err(e) => {
                        warn_log!("读取在线 API 响应失败: {}, 使用默认 IP", e);
                        DEFAULT_CF_IPS.iter().map(|s| s.to_string()).collect()
                    }
                }
            } else {
                warn_log!("在线 API 返回状态码 {}, 使用默认 IP", resp.status());
                DEFAULT_CF_IPS.iter().map(|s| s.to_string()).collect()
            }
        }
        Err(e) => {
            warn_log!("请求在线 API 失败: {}, 使用默认 IP", e);
            DEFAULT_CF_IPS.iter().map(|s| s.to_string()).collect()
        }
    }
}

/// Reusable endpoint tester with connection pooling
#[derive(Clone)]
pub struct EndpointTester {
    custom_cf_ips: Arc<Vec<String>>,
    online_cf_ips: Arc<Mutex<Option<Vec<String>>>>,
    cancelled: Arc<AtomicBool>,
    resolver: Arc<TokioAsyncResolver>,
    tls_connector: TlsConnector,
    /// 每个 IP 测试的轮次（取中位数以提高准确性）
    test_rounds: u32,
}

use tokio::sync::Mutex;

impl EndpointTester {
    pub fn new(custom_cf_ips: Vec<String>, test_rounds: u32) -> Self {
        // Install ring as the default CryptoProvider (safe to call multiple times;
        // needed when both ring and aws-lc-rs features are enabled via deps)
        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

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

        // Clamp test rounds to 1..=5
        let test_rounds = test_rounds.clamp(1, 5);

        Self {
            custom_cf_ips: Arc::new(custom_cf_ips),
            online_cf_ips: Arc::new(Mutex::new(None)),
            cancelled: Arc::new(AtomicBool::new(false)),
            resolver: Arc::new(resolver),
            tls_connector,
            test_rounds,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub async fn test_ip(&self, endpoint: &Endpoint, ip: String) -> EndpointResult {
        self.test_single_ip(endpoint, ip).await
    }

    /// Get CF IPs: custom > online API > default fallback
    async fn get_cf_ips(&self) -> Vec<String> {
        // 1. 如果用户配置了自定义 IP，优先使用
        if !self.custom_cf_ips.is_empty() {
            debug_log!("使用用户自定义 CF IP ({} 个)", self.custom_cf_ips.len());
            return self.custom_cf_ips.to_vec();
        }

        // 2. 尝试使用缓存的在线 IP
        {
            let cached = self.online_cf_ips.lock().await;
            if let Some(ips) = cached.as_ref() {
                debug_log!("使用缓存的在线优选 IP ({} 个)", ips.len());
                return ips.clone();
            }
        }

        // 3. 从在线 API 获取并缓存
        let online_ips = fetch_online_cf_ips().await;
        {
            let mut cached = self.online_cf_ips.lock().await;
            *cached = Some(online_ips.clone());
        }
        online_ips
    }

    /// Test all endpoints concurrently with controlled parallelism
    pub async fn test_all(&self, endpoints: &[Endpoint]) -> Vec<EndpointResult> {
        info_log!("开始测试 {} 个端点", endpoints.len());

        if endpoints.is_empty() {
            warn_log!("端点列表为空，直接返回");
            return Vec::new();
        }

        // Limit concurrent endpoint tests to avoid overwhelming the system
        let max_concurrency = endpoints.len().clamp(1, MAX_ENDPOINT_CONCURRENCY);
        debug_log!("最大并发数: {}", max_concurrency);
        let semaphore = Arc::new(Semaphore::new(max_concurrency));
        let mut join_set = JoinSet::new();

        // 追踪已 spawn 的端点，用于处理 panic 情况
        let mut spawned_endpoints: Vec<Endpoint> = Vec::new();

        for (idx, endpoint) in endpoints.iter().enumerate() {
            if self.cancelled.load(Ordering::SeqCst) {
                warn_log!("测试已取消，停止添加新任务");
                break;
            }

            debug_log!(
                "[{}/{}] 准备测试端点: {} ({})",
                idx + 1,
                endpoints.len(),
                endpoint.name,
                endpoint.domain
            );

            let endpoint = endpoint.clone();
            let tester = self.clone();

            // 获取信号量，添加 5 秒超时防止死锁
            let permit = match tokio::time::timeout(
                Duration::from_secs(5),
                semaphore.clone().acquire_owned(),
            )
            .await
            {
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

            // 记录已 spawn 的端点
            spawned_endpoints.push(endpoint.clone());

            let idx_copy = idx;
            let total = endpoints.len();
            join_set.spawn(async move {
                let _permit = permit;
                debug_log!("[{}/{}] 开始测试: {}", idx_copy + 1, total, endpoint.name);
                let start = Instant::now();
                let result = tester.test_endpoint(&endpoint).await;
                debug_log!(
                    "[{}/{}] 测试完成: {} - {} (耗时 {:.1}s)",
                    idx_copy + 1,
                    total,
                    endpoint.name,
                    if result.success {
                        format!("成功 {:.0}ms", result.latency)
                    } else {
                        format!("失败: {}", result.error.as_deref().unwrap_or("unknown"))
                    },
                    start.elapsed().as_secs_f64()
                );
                result
            });
        }

        info_log!("已添加 {} 个测试任务，等待完成...", join_set.len());

        let mut results = Vec::with_capacity(endpoints.len());
        let collect_start = Instant::now();
        let collect_timeout =
            estimate_test_timeout(spawned_endpoints.len()).saturating_sub(COLLECT_TIMEOUT_HEADROOM);
        let mut panic_count = 0usize;

        // 收集结果，使用动态预算而不是固定 30 秒，避免后排端点饥饿
        loop {
            // 检查总体超时
            if collect_start.elapsed() > collect_timeout {
                warn_log!(
                    "收集结果超时（{}秒），已收集 {} 个结果，中止剩余任务",
                    collect_timeout.as_secs(),
                    results.len(),
                );
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
                    debug_log!(
                        "已收集 {}/{} 个结果",
                        results.len(),
                        spawned_endpoints.len()
                    );
                }
                Ok(Some(Err(e))) => {
                    panic_count += 1;
                    error_log!("任务 panic: {:?}", e);
                }
                Ok(None) => {
                    // 所有任务完成
                    info_log!(
                        "所有任务完成，共 {} 个结果，{} 个 panic",
                        results.len(),
                        panic_count
                    );
                    break;
                }
                Err(_) => {
                    // 单个等待超时，继续等待
                    debug_log!("等待下一个结果超时，继续...");
                }
            }
        }

        // 为没有返回结果的端点（panic 或超时）创建失败记录
        let returned_domains: std::collections::HashSet<String> =
            results.iter().map(|r| r.endpoint.domain.clone()).collect();

        for endpoint in &spawned_endpoints {
            if !returned_domains.contains(&endpoint.domain) {
                warn_log!(
                    "端点 {} ({}) 测试异常，未返回结果",
                    endpoint.name,
                    endpoint.domain
                );
                results.push(EndpointResult::failure(
                    endpoint.clone(),
                    String::new(),
                    "测试异常（任务崩溃或超时）".into(),
                ));
            }
        }

        // Sort by latency (成功的排前面，失败的排后面)
        results.sort_by(|a, b| match (a.success, b.success) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .latency
                .partial_cmp(&b.latency)
                .unwrap_or(std::cmp::Ordering::Equal),
        });

        let success_count = results.iter().filter(|r| r.success).count();
        info_log!(
            "测速完成: {}/{} 成功, 最佳延迟: {:.0}ms",
            success_count,
            results.len(),
            results.first().map(|r| r.latency).unwrap_or(0.0)
        );

        results
    }

    /// Test a single endpoint and find the best IP
    pub async fn test_endpoint(&self, endpoint: &Endpoint) -> EndpointResult {
        debug_log!(
            "test_endpoint 开始: {} ({})",
            endpoint.name,
            endpoint.domain
        );

        if self.cancelled.load(Ordering::SeqCst) {
            warn_log!("test_endpoint: 检测到取消信号");
            return EndpointResult::failure(endpoint.clone(), String::new(), "已取消".into());
        }

        // Resolve DNS using cached resolver
        debug_log!("  DNS 解析: {}", endpoint.domain);
        let dns_start = Instant::now();
        let dns_result = tokio::time::timeout(
            DNS_LOOKUP_TIMEOUT,
            self.resolver.lookup_ip(&endpoint.domain),
        )
        .await;

        let dns_ips: Vec<String> = match dns_result {
            Ok(Ok(lookup)) => {
                let ips: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                debug_log!(
                    "  DNS 成功 ({:.1}ms): {} 个 IP - {:?}",
                    dns_start.elapsed().as_secs_f64() * 1000.0,
                    ips.len(),
                    ips
                );
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
                error_log!("  DNS 超时 ({}s)", DNS_LOOKUP_TIMEOUT.as_secs());
                return EndpointResult::failure(endpoint.clone(), String::new(), "DNS超时".into());
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
            debug_log!(
                "  原始 IP 失败: {}",
                original_result.error.as_deref().unwrap_or("unknown")
            );
            9999.0
        };

        // Check if Cloudflare
        let is_cf = dns_ips.iter().any(|ip| is_cloudflare_ip(ip));
        if is_cf {
            debug_log!("  检测到 Cloudflare IP，启用 CF 优选");
        }

        // Collect IPs to test
        // User-configured preferred IPs take highest priority regardless of CF detection
        let test_ips: Vec<String> = if !self.custom_cf_ips.is_empty() {
            debug_log!("  使用用户白名单 IP（优先级最高），不合并 DNS IP");
            self.custom_cf_ips.to_vec()
        } else if is_cf {
            let cf_ips = self.get_cf_ips().await;
            merge_candidate_ips(cf_ips, &dns_ips, MAX_TEST_IPS)
        } else {
            // 非 CF 站点：并发查询多个公共 DNS，收集更多候选 IP
            debug_log!("  非CF站点，启用多DNS解析器优选");
            let multi_dns_ips = resolve_via_multi_dns(&endpoint.domain).await;
            if multi_dns_ips.len() > dns_ips.len() {
                debug_log!(
                    "  多DNS解析发现 {} 个唯一IP（原DNS {} 个）",
                    multi_dns_ips.len(),
                    dns_ips.len()
                );
            }
            // 合并：DNS IP 优先，然后追加多 DNS 发现的新 IP，限制总数
            let mut seen = HashSet::new();
            let mut merged = Vec::with_capacity(MAX_TEST_IPS);
            for ip in dns_ips.iter().chain(multi_dns_ips.iter()) {
                if seen.insert(ip.clone()) {
                    merged.push(ip.clone());
                    if merged.len() >= MAX_TEST_IPS {
                        break;
                    }
                }
            }
            merged
        };

        debug_log!("  准备测试 {} 个 IP", test_ips.len());

        // Test all IPs concurrently with timeout
        let mut join_set = JoinSet::new();
        let ip_semaphore = Arc::new(Semaphore::new(MAX_IP_CONCURRENCY_PER_ENDPOINT));
        for ip in test_ips.iter() {
            let ep = endpoint.clone();
            let tester = self.clone();
            let ip_clone = ip.clone();
            let ip_permit = ip_semaphore.clone();
            join_set.spawn(async move {
                match ip_permit.acquire_owned().await {
                    Ok(_permit) => tester.test_single_ip(&ep, ip_clone).await,
                    Err(_) => EndpointResult::failure(ep, ip_clone, "并发控制异常".into()),
                }
            });
        }

        // Collect results with 15s total timeout for all IP tests
        let mut best_result: Option<EndpointResult> = None;
        let ip_test_start = Instant::now();
        let ip_test_timeout = IP_TEST_TOTAL_TIMEOUT;

        loop {
            // 检查总超时
            if ip_test_start.elapsed() > ip_test_timeout {
                warn_log!(
                    "  IP 测试超时 ({}s)，已测试部分 IP",
                    IP_TEST_TOTAL_TIMEOUT.as_secs()
                );
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
                        if best_result.is_none()
                            || result.latency < best_result.as_ref().unwrap().latency
                        {
                            debug_log!(
                                "    IP {} 延迟 {:.0}ms (新最优)",
                                result.ip,
                                result.latency
                            );
                            best_result = Some(result);
                        } else {
                            debug_log!("    IP {} 延迟 {:.0}ms", result.ip, result.latency);
                        }
                    } else {
                        debug_log!(
                            "    IP {} 失败: {}",
                            result.ip,
                            result.error.as_deref().unwrap_or("unknown")
                        );
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
            info_log!(
                "  端点 {} 最优 IP: {} ({:.0}ms, 原 {:.0}ms)",
                endpoint.name,
                best.ip,
                best.latency,
                original_latency
            );
            EndpointResult::success_with_comparison(
                endpoint.clone(),
                best.ip,
                best.latency,
                original_ip,
                original_latency,
            )
        } else if original_result.success {
            // 如果优化 IP 都失败，但原始 IP 成功，使用原始 IP
            info_log!(
                "  端点 {} 使用原始 IP: {} ({:.0}ms)",
                endpoint.name,
                original_result.ip,
                original_result.latency
            );
            let mut result = EndpointResult::success_with_comparison(
                endpoint.clone(),
                original_result.ip.clone(),
                original_result.latency,
                original_ip.clone(),
                original_latency,
            );

            // 用户设置了优选 IP 白名单但全部失败，设置警告
            if !self.custom_cf_ips.is_empty() {
                let mut warning = "优选IP全部失败，已回退至DNS默认IP".to_string();
                if !is_cf {
                    warning.push_str("。该域名非Cloudflare域名，CF优选IP不适用");
                }
                warn_log!("  端点 {} 警告: {}", endpoint.name, warning);
                result.warning = Some(warning);
            }

            result
        } else {
            error_log!("  端点 {} 全部失败", endpoint.name);
            EndpointResult::failure(endpoint.clone(), original_ip, "全部超时".into())
        };

        debug_log!("test_endpoint 完成: {}", endpoint.name);
        final_result
    }

    async fn test_single_ip(&self, endpoint: &Endpoint, ip: String) -> EndpointResult {
        let rounds = self.test_rounds as usize;
        let mut latencies: Vec<f64> = Vec::with_capacity(rounds);

        for round in 0..rounds {
            match tokio::time::timeout(SINGLE_IP_TEST_TIMEOUT, self.do_https_test(endpoint, &ip))
                .await
            {
                Ok(Ok(latency)) => {
                    latencies.push(latency);
                }
                Ok(Err(_)) | Err(_) => {
                    // 首轮失败直接放弃（IP 大概率不可达）
                    if round == 0 {
                        return EndpointResult::failure(
                            endpoint.clone(),
                            ip,
                            "首轮测试失败".into(),
                        );
                    }
                    // 后续轮次失败忽略，用已有数据
                }
            }
        }

        if latencies.is_empty() {
            return EndpointResult::failure(endpoint.clone(), ip, "全部超时".into());
        }

        // 取中位数（排序后取中间值，抗抖动）
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = latencies[latencies.len() / 2];

        EndpointResult::success(endpoint.clone(), ip, median)
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
        let domain = endpoint
            .domain
            .clone()
            .try_into()
            .map_err(|_| "Invalid domain")?;

        let mut tls_stream = connector
            .connect(domain, stream)
            .await
            .map_err(|e| format!("TLS: {}", e))?;

        // Always test with "/" - we only need to verify IP connectivity (TCP+TLS+HTTP),
        // not the actual API path (e.g. /v1) which often requires authentication and times out
        let request = format!(
            "HEAD / HTTP/1.1\r\nHost: {}\r\nUser-Agent: anyrouter/1.0\r\nConnection: close\r\n\r\n",
            endpoint.domain
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
        assert!(!is_cloudflare_ip("104.15.0.1")); // Close but not CF
        assert!(!is_cloudflare_ip("104.28.0.1")); // Close but not CF
    }

    #[test]
    fn test_default_cf_ips_are_valid() {
        for ip in DEFAULT_CF_IPS {
            // All default CF IPs should be recognized as CF IPs
            assert!(is_cloudflare_ip(ip), "IP {} should be recognized as CF", ip);
        }
    }

    #[test]
    fn test_merge_candidate_ips_keeps_order_and_dedupes() {
        let cf_ips = vec![
            "1.1.1.1".to_string(),
            "2.2.2.2".to_string(),
            "1.1.1.1".to_string(),
            "3.3.3.3".to_string(),
        ];
        let dns_ips = vec!["2.2.2.2".to_string(), "4.4.4.4".to_string()];

        let merged = merge_candidate_ips(cf_ips, &dns_ips, 10);
        assert_eq!(merged, vec!["1.1.1.1", "2.2.2.2", "3.3.3.3", "4.4.4.4"]);
    }

    #[test]
    fn test_merge_candidate_ips_respects_limit() {
        let cf_ips = vec![
            "1.1.1.1".to_string(),
            "2.2.2.2".to_string(),
            "3.3.3.3".to_string(),
        ];
        let dns_ips = vec!["4.4.4.4".to_string()];

        let merged = merge_candidate_ips(cf_ips, &dns_ips, 2);
        assert_eq!(merged, vec!["1.1.1.1", "2.2.2.2"]);
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
        let tester = EndpointTester::new(vec![], 3);

        // Verify it can be cloned (required for concurrent testing)
        let _cloned = tester.clone();
        assert_eq!(tester.test_rounds, 3);
    }

    #[tokio::test]
    async fn test_endpoint_tester_with_custom_ips() {
        let custom_ips = vec!["1.2.3.4".to_string()];
        let tester = EndpointTester::new(custom_ips.clone(), 3);

        // Verify custom IPs are stored
        assert_eq!(tester.custom_cf_ips.len(), 1);
        assert_eq!(tester.custom_cf_ips[0], "1.2.3.4");
    }

    #[tokio::test]
    async fn test_endpoint_tester_cancel() {
        let tester = EndpointTester::new(vec![], 3);

        // Initially not cancelled
        assert!(!tester.cancelled.load(Ordering::SeqCst));

        // After cancel
        tester.cancel();
        assert!(tester.cancelled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_endpoint_tester_test_rounds_clamping() {
        let tester_low = EndpointTester::new(vec![], 0);
        assert_eq!(tester_low.test_rounds, 1);

        let tester_high = EndpointTester::new(vec![], 10);
        assert_eq!(tester_high.test_rounds, 5);

        let tester_normal = EndpointTester::new(vec![], 3);
        assert_eq!(tester_normal.test_rounds, 3);
    }

    #[tokio::test]
    async fn test_test_all_empty_endpoints() {
        let tester = EndpointTester::new(vec![], 3);
        let results = tester.test_all(&[]).await;

        assert!(results.is_empty());
    }
}
