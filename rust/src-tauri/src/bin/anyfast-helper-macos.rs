//! anyFAST macOS Privilege Helper
//!
//! A minimal setuid helper binary for modifying /etc/hosts on macOS.
//! This binary should be owned by root with setuid bit set:
//!   sudo chown root:wheel anyfast-helper-macos
//!   sudo chmod 4755 anyfast-helper-macos
//!
//! Usage:
//!   anyfast-helper-macos write <domain> <ip>
//!   anyfast-helper-macos write-batch <json_bindings>
//!   anyfast-helper-macos clear <domain>
//!   anyfast-helper-macos clear-batch <json_domains>
//!   anyfast-helper-macos clear-all
//!   anyfast-helper-macos flush-dns

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::Path;
use std::process::{Command, ExitCode};

const HOSTS_PATH: &str = "/etc/hosts";
const MARKER_BEGIN: &str = "# BEGIN anyFAST";
const MARKER_END: &str = "# END anyFAST";
const MARKER_LINE: &str = "# anyFAST";

fn main() -> ExitCode {
    // Explicitly set effective UID to root (required for setuid to work)
    #[cfg(unix)]
    unsafe {
        if libc::setuid(0) != 0 {
            eprintln!("错误: 无法获取 root 权限，请确保 helper 已正确设置 setuid");
            return ExitCode::from(1);
        }
    }

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return ExitCode::from(1);
    }

    let result = match args[1].as_str() {
        "write" => {
            if args.len() != 4 {
                eprintln!("用法: {} write <domain> <ip>", args[0]);
                return ExitCode::from(1);
            }
            write_binding(&args[2], &args[3])
        }
        "write-batch" => {
            if args.len() != 3 {
                eprintln!("用法: {} write-batch <json_bindings>", args[0]);
                return ExitCode::from(1);
            }
            write_bindings_batch(&args[2])
        }
        "clear" => {
            if args.len() != 3 {
                eprintln!("用法: {} clear <domain>", args[0]);
                return ExitCode::from(1);
            }
            clear_binding(&args[2])
        }
        "clear-batch" => {
            if args.len() != 3 {
                eprintln!("用法: {} clear-batch <json_domains>", args[0]);
                return ExitCode::from(1);
            }
            clear_bindings_batch(&args[2])
        }
        "clear-all" => clear_all_anyfast_bindings(),
        "flush-dns" => flush_dns(),
        _ => {
            print_usage();
            return ExitCode::from(1);
        }
    };

    match result {
        Ok(msg) => {
            println!("{}", msg);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("错误: {}", e);
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    eprintln!("anyFAST macOS Privilege Helper");
    eprintln!("用法:");
    eprintln!("  anyfast-helper-macos write <domain> <ip>");
    eprintln!("  anyfast-helper-macos write-batch <json_bindings>");
    eprintln!("  anyfast-helper-macos clear <domain>");
    eprintln!("  anyfast-helper-macos clear-batch <json_domains>");
    eprintln!("  anyfast-helper-macos clear-all");
    eprintln!("  anyfast-helper-macos flush-dns");
}

// ============ Validation ============

fn validate_ip(ip: &str) -> Result<(), String> {
    ip.parse::<IpAddr>()
        .map_err(|_| format!("无效的 IP 地址: {}", ip))?;
    Ok(())
}

fn validate_domain(domain: &str) -> Result<(), String> {
    if domain.is_empty() {
        return Err("域名不能为空".to_string());
    }
    if domain.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!("域名包含无效字符: {}", domain));
    }
    if !domain
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_')
    {
        return Err(format!("无效的域名格式: {}", domain));
    }
    Ok(())
}

// ============ Hosts File Parsing ============

struct ParsedHosts {
    before_block: Vec<String>,
    after_block: Vec<String>,
    anyfast_bindings: HashMap<String, String>,
}

impl ParsedHosts {
    fn parse(content: &str) -> Self {
        let mut before_block = Vec::new();
        let mut after_block = Vec::new();
        let mut anyfast_bindings = HashMap::new();

        let mut in_block = false;
        let mut found_block = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == MARKER_BEGIN {
                in_block = true;
                found_block = true;
                continue;
            }

            if trimmed == MARKER_END {
                in_block = false;
                continue;
            }

            if in_block {
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        anyfast_bindings.insert(parts[1].to_string(), parts[0].to_string());
                    }
                }
            } else if found_block {
                after_block.push(line.to_string());
            } else {
                // Check for legacy line-level markers
                if trimmed.contains(MARKER_LINE) && !trimmed.is_empty() && !trimmed.starts_with('#')
                {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        anyfast_bindings.insert(parts[1].to_string(), parts[0].to_string());
                    }
                } else {
                    before_block.push(line.to_string());
                }
            }
        }

        ParsedHosts {
            before_block,
            after_block,
            anyfast_bindings,
        }
    }

    fn render(&self) -> String {
        let mut lines = self.before_block.clone();

        if !self.anyfast_bindings.is_empty() {
            if !lines.is_empty() && !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
                lines.push(String::new());
            }

            lines.push(MARKER_BEGIN.to_string());

            let mut sorted_bindings: Vec<_> = self.anyfast_bindings.iter().collect();
            sorted_bindings.sort_by_key(|(domain, _)| *domain);

            for (domain, ip) in sorted_bindings {
                lines.push(format!("{}\t{}\t{}", ip, domain, MARKER_LINE));
            }

            lines.push(MARKER_END.to_string());
        }

        lines.extend(self.after_block.clone());
        lines.join("\n")
    }
}

// ============ File Operations ============

fn read_hosts_content() -> Result<String, String> {
    let mut file = File::open(HOSTS_PATH).map_err(|e| format!("无法打开 hosts 文件: {}", e))?;

    let mut raw_content = Vec::new();
    file.read_to_end(&mut raw_content)
        .map_err(|e| format!("无法读取 hosts 文件: {}", e))?;

    // Handle UTF-8 BOM
    let content = if raw_content.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&raw_content[3..]).to_string()
    } else {
        String::from_utf8_lossy(&raw_content).to_string()
    };

    Ok(content)
}

fn atomic_write(content: &str) -> Result<(), String> {
    let path = Path::new(HOSTS_PATH);
    let parent = path.parent().unwrap_or(Path::new("/etc"));
    let temp_path = parent.join(format!(".hosts.tmp.{}", std::process::id()));

    // Write to temp file
    {
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| format!("无法创建临时文件: {}", e))?;

        temp_file
            .write_all(content.as_bytes())
            .map_err(|e| format!("无法写入临时文件: {}", e))?;
        temp_file
            .flush()
            .map_err(|e| format!("无法刷新临时文件: {}", e))?;
        temp_file
            .sync_all()
            .map_err(|e| format!("无法同步临时文件: {}", e))?;
    }

    // Atomic rename
    fs::rename(&temp_path, path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        format!("无法重命名临时文件: {}", e)
    })?;

    Ok(())
}

// ============ Commands ============

fn write_binding(domain: &str, ip: &str) -> Result<String, String> {
    validate_ip(ip)?;
    validate_domain(domain)?;

    let content = read_hosts_content()?;
    let mut parsed = ParsedHosts::parse(&content);

    parsed
        .anyfast_bindings
        .insert(domain.to_string(), ip.to_string());

    let new_content = parsed.render();
    atomic_write(&new_content)?;

    Ok(format!("已写入: {} -> {}", domain, ip))
}

fn write_bindings_batch(json_bindings: &str) -> Result<String, String> {
    // Parse JSON: [["domain1", "ip1"], ["domain2", "ip2"], ...]
    let bindings: Vec<Vec<String>> =
        serde_json::from_str(json_bindings).map_err(|e| format!("无效的 JSON 格式: {}", e))?;

    // Validate all inputs first
    for binding in &bindings {
        if binding.len() != 2 {
            return Err("每个绑定必须包含 [domain, ip]".to_string());
        }
        validate_domain(&binding[0])?;
        validate_ip(&binding[1])?;
    }

    let content = read_hosts_content()?;
    let mut parsed = ParsedHosts::parse(&content);

    let mut count = 0;
    for binding in &bindings {
        parsed
            .anyfast_bindings
            .insert(binding[0].clone(), binding[1].clone());
        count += 1;
    }

    let new_content = parsed.render();
    atomic_write(&new_content)?;

    Ok(format!("已写入 {} 条绑定", count))
}

fn clear_binding(domain: &str) -> Result<String, String> {
    let content = read_hosts_content()?;
    let mut parsed = ParsedHosts::parse(&content);

    if parsed.anyfast_bindings.remove(domain).is_some() {
        let new_content = parsed.render();
        atomic_write(&new_content)?;
        Ok(format!("已清除: {}", domain))
    } else {
        Ok(format!("未找到: {}", domain))
    }
}

fn clear_bindings_batch(json_domains: &str) -> Result<String, String> {
    // Parse JSON: ["domain1", "domain2", ...]
    let domains: Vec<String> =
        serde_json::from_str(json_domains).map_err(|e| format!("无效的 JSON 格式: {}", e))?;

    let content = read_hosts_content()?;
    let mut parsed = ParsedHosts::parse(&content);

    let domains_set: HashSet<&str> = domains.iter().map(|s| s.as_str()).collect();
    let mut removed_count = 0;

    for domain in &domains_set {
        if parsed.anyfast_bindings.remove(*domain).is_some() {
            removed_count += 1;
        }
    }

    let new_content = parsed.render();
    atomic_write(&new_content)?;

    Ok(format!("已清除 {} 条绑定", removed_count))
}

fn clear_all_anyfast_bindings() -> Result<String, String> {
    let content = read_hosts_content()?;
    let mut parsed = ParsedHosts::parse(&content);

    let removed_count = parsed.anyfast_bindings.len();
    parsed.anyfast_bindings.clear();

    let new_content = parsed.render();
    atomic_write(&new_content)?;

    Ok(format!("已清除所有 anyFAST 绑定 ({} 条)", removed_count))
}

fn flush_dns() -> Result<String, String> {
    // macOS DNS cache flush
    Command::new("/usr/bin/dscacheutil")
        .args(["-flushcache"])
        .output()
        .map_err(|e| format!("无法执行 dscacheutil: {}", e))?;

    // Also kill mDNSResponder to ensure cache is fully cleared
    let _ = Command::new("/usr/bin/killall")
        .args(["-HUP", "mDNSResponder"])
        .output();

    Ok("DNS 缓存已刷新".to_string())
}
