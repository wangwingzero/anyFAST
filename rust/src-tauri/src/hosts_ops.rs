//! Hosts operations with Service/Helper fallback
//!
//! This module provides a unified interface for hosts file operations:
//! - Windows: Uses Windows Service when available, falls back to direct operations
//! - macOS: Uses setuid helper binary for privilege elevation
//! - Linux: Falls back to direct operations (requires root)

use crate::hosts_manager::{HostsBinding, HostsError, HostsManager};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[cfg(windows)]
use crate::client::PipeClient;

#[cfg(target_os = "macos")]
use std::process::Command;

#[cfg(target_os = "macos")]
use std::sync::RwLock;

/// Cached state of whether the service is running
#[allow(dead_code)]
static SERVICE_AVAILABLE: OnceLock<AtomicBool> = OnceLock::new();

/// Cached path to macOS helper binary (installed with setuid)
/// Using RwLock to allow refreshing after installation
#[cfg(target_os = "macos")]
static MACOS_HELPER_PATH: OnceLock<RwLock<Option<std::path::PathBuf>>> = OnceLock::new();

/// Flag to force re-check of macOS helper (set after installation)
#[cfg(target_os = "macos")]
static MACOS_HELPER_NEEDS_REFRESH: AtomicBool = AtomicBool::new(false);

/// Path where helper should be installed
#[cfg(target_os = "macos")]
const MACOS_HELPER_INSTALL_PATH: &str = "/usr/local/bin/anyfast-helper-macos";

/// Get the path to the bundled helper binary (inside app bundle, without setuid)
#[cfg(target_os = "macos")]
pub fn get_bundled_helper_path() -> Option<std::path::PathBuf> {
    let possible_paths = [
        // Inside app bundle MacOS directory
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("anyfast-helper-macos"))),
        // Inside app bundle Resources directory
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .and_then(|p| p.parent())
                .map(|p| p.join("Resources/anyfast-helper-macos"))
        }),
        // Development: target directory
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("anyfast-helper-macos"))),
    ];

    for path_opt in possible_paths.into_iter().flatten() {
        if path_opt.exists() {
            return Some(path_opt);
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn get_bundled_helper_path() -> Option<std::path::PathBuf> {
    None
}

/// Check if macOS helper exists and has setuid bit
#[cfg(target_os = "macos")]
fn check_macos_helper_internal() -> Option<std::path::PathBuf> {
    // Only check the installed location - it must have setuid bit
    let install_path = std::path::PathBuf::from(MACOS_HELPER_INSTALL_PATH);

    if install_path.exists() {
        // Check if it has setuid bit set
        use std::os::unix::fs::MetadataExt;
        if let Ok(metadata) = std::fs::metadata(&install_path) {
            let mode = metadata.mode();
            // Check for setuid bit (0o4000)
            if mode & 0o4000 != 0 {
                return Some(install_path);
            } else {
                eprintln!("警告: helper 存在但未设置 setuid 位: {:?}", install_path);
            }
        }
    }
    None
}

/// Get the path to the installed macOS helper binary (with setuid bit set)
#[cfg(target_os = "macos")]
fn get_macos_helper_path() -> Option<std::path::PathBuf> {
    let lock = MACOS_HELPER_PATH.get_or_init(|| RwLock::new(check_macos_helper_internal()));

    // Check if we need to refresh (after installation)
    if MACOS_HELPER_NEEDS_REFRESH.swap(false, Ordering::SeqCst) {
        if let Ok(mut guard) = lock.write() {
            *guard = check_macos_helper_internal();
        }
    }

    lock.read().ok().and_then(|guard| guard.clone())
}

/// Refresh macOS helper status (call after installation)
#[cfg(target_os = "macos")]
pub fn refresh_macos_helper_status() -> bool {
    MACOS_HELPER_NEEDS_REFRESH.store(true, Ordering::SeqCst);
    is_macos_helper_available()
}

#[cfg(not(target_os = "macos"))]
pub fn refresh_macos_helper_status() -> bool {
    false
}

/// Check if macOS helper is available and properly configured
#[cfg(target_os = "macos")]
pub fn is_macos_helper_available() -> bool {
    get_macos_helper_path().is_some()
}

#[cfg(not(target_os = "macos"))]
pub fn is_macos_helper_available() -> bool {
    false
}

/// Check if the hosts service is running (cached)
#[cfg(windows)]
pub fn is_service_running() -> bool {
    let available = SERVICE_AVAILABLE.get_or_init(|| {
        let client = PipeClient::new();
        AtomicBool::new(client.is_service_running())
    });
    available.load(Ordering::Relaxed)
}

#[cfg(not(windows))]
pub fn is_service_running() -> bool {
    false
}

/// Refresh the service availability check
#[cfg(windows)]
pub fn refresh_service_status() -> bool {
    let client = PipeClient::new();
    let running = client.is_service_running();
    // 更新缓存状态
    if let Some(available) = SERVICE_AVAILABLE.get() {
        available.store(running, Ordering::Relaxed);
    } else {
        // 如果还没初始化，初始化它
        SERVICE_AVAILABLE.get_or_init(|| AtomicBool::new(running));
    }
    running
}

#[cfg(not(windows))]
pub fn refresh_service_status() -> bool {
    false
}

/// Mark service as unavailable (called on service failure)
#[allow(dead_code)]
fn mark_service_unavailable() {
    if let Some(available) = SERVICE_AVAILABLE.get() {
        available.store(false, Ordering::Relaxed);
    }
}

/// Write a binding using Service if available, otherwise direct
/// On service failure, automatically falls back to direct operation
pub fn write_binding(domain: &str, ip: &str) -> Result<(), HostsError> {
    #[cfg(windows)]
    {
        if is_service_running() {
            let client = PipeClient::new();
            match client.write_binding(domain, ip) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // Service failed - mark unavailable and fall back to direct
                    eprintln!(
                        "Service write_binding failed, falling back to direct: {}",
                        e
                    );
                    mark_service_unavailable();
                    // Fall through to direct operation
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(helper_path) = get_macos_helper_path() {
            match Command::new(&helper_path)
                .args(["write", domain, ip])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        return Ok(());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("macOS helper write_binding failed: {}", stderr);
                        // Fall through to direct operation
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute macOS helper: {}", e);
                    // Fall through to direct operation
                }
            }
        }
    }

    // Fall back to direct operation
    // If this also fails with PermissionDenied, the error will propagate up
    // and the frontend should prompt for admin restart
    HostsManager::write_binding(domain, ip)
}

/// Write multiple bindings using Service if available, otherwise direct
/// On service failure, automatically falls back to direct operation
pub fn write_bindings_batch(bindings: &[HostsBinding]) -> Result<usize, HostsError> {
    #[cfg(windows)]
    {
        if is_service_running() {
            let client = PipeClient::new();
            let binding_tuples: Vec<(String, String)> = bindings
                .iter()
                .map(|b| (b.domain.clone(), b.ip.clone()))
                .collect();

            match client.write_bindings_batch(&binding_tuples) {
                Ok(count) => return Ok(count as usize),
                Err(e) => {
                    // Service failed - mark unavailable and fall back to direct
                    eprintln!(
                        "Service write_bindings_batch failed, falling back to direct: {}",
                        e
                    );
                    mark_service_unavailable();
                    // Fall through to direct operation
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(helper_path) = get_macos_helper_path() {
            // Convert bindings to JSON: [["domain1", "ip1"], ["domain2", "ip2"], ...]
            let json_bindings: Vec<Vec<&str>> = bindings
                .iter()
                .map(|b| vec![b.domain.as_str(), b.ip.as_str()])
                .collect();
            let json_str = serde_json::to_string(&json_bindings).unwrap_or_default();

            match Command::new(&helper_path)
                .args(["write-batch", &json_str])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        return Ok(bindings.len());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("macOS helper write_bindings_batch failed: {}", stderr);
                        // Fall through to direct operation
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute macOS helper: {}", e);
                    // Fall through to direct operation
                }
            }
        }
    }

    HostsManager::write_bindings_batch(bindings)
}

/// Clear a binding using Service if available, otherwise direct
/// On service failure, automatically falls back to direct operation
#[allow(dead_code)]
pub fn clear_binding(domain: &str) -> Result<(), HostsError> {
    #[cfg(windows)]
    {
        if is_service_running() {
            let client = PipeClient::new();
            match client.clear_binding(domain) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // Service failed - mark unavailable and fall back to direct
                    eprintln!(
                        "Service clear_binding failed, falling back to direct: {}",
                        e
                    );
                    mark_service_unavailable();
                    // Fall through to direct operation
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(helper_path) = get_macos_helper_path() {
            match Command::new(&helper_path).args(["clear", domain]).output() {
                Ok(output) => {
                    if output.status.success() {
                        return Ok(());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("macOS helper clear_binding failed: {}", stderr);
                        // Fall through to direct operation
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute macOS helper: {}", e);
                    // Fall through to direct operation
                }
            }
        }
    }

    HostsManager::clear_binding(domain)
}

/// Clear multiple bindings using Service if available, otherwise direct
/// On service failure, automatically falls back to direct operation
pub fn clear_bindings_batch(domains: &[&str]) -> Result<usize, HostsError> {
    #[cfg(windows)]
    {
        if is_service_running() {
            let client = PipeClient::new();
            let domain_strings: Vec<String> = domains.iter().map(|s| s.to_string()).collect();

            match client.clear_bindings_batch(&domain_strings) {
                Ok(count) => return Ok(count as usize),
                Err(e) => {
                    // Service failed - mark unavailable and fall back to direct
                    eprintln!(
                        "Service clear_bindings_batch failed, falling back to direct: {}",
                        e
                    );
                    mark_service_unavailable();
                    // Fall through to direct operation
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(helper_path) = get_macos_helper_path() {
            let json_str = serde_json::to_string(&domains).unwrap_or_default();

            match Command::new(&helper_path)
                .args(["clear-batch", &json_str])
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        return Ok(domains.len());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("macOS helper clear_bindings_batch failed: {}", stderr);
                        // Fall through to direct operation
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute macOS helper: {}", e);
                    // Fall through to direct operation
                }
            }
        }
    }

    HostsManager::clear_bindings_batch(domains)
}

/// Clear ALL anyFAST-managed bindings using Service if available, otherwise direct
/// This removes the entire anyFAST block regardless of current config
/// On service failure, automatically falls back to direct operation
pub fn clear_all_anyfast_bindings() -> Result<usize, HostsError> {
    #[cfg(windows)]
    {
        if is_service_running() {
            let client = PipeClient::new();

            match client.clear_all_anyfast_bindings() {
                Ok(count) => return Ok(count as usize),
                Err(e) => {
                    // Service failed - mark unavailable and fall back to direct
                    eprintln!(
                        "Service clear_all_anyfast_bindings failed, falling back to direct: {}",
                        e
                    );
                    mark_service_unavailable();
                    // Fall through to direct operation
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(helper_path) = get_macos_helper_path() {
            match Command::new(&helper_path).args(["clear-all"]).output() {
                Ok(output) => {
                    if output.status.success() {
                        // Parse the count from output if needed, or return 0
                        return Ok(0);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("macOS helper clear_all_anyfast_bindings failed: {}", stderr);
                        // Fall through to direct operation
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute macOS helper: {}", e);
                    // Fall through to direct operation
                }
            }
        }
    }

    HostsManager::clear_all_anyfast_bindings()
}

/// Read a binding (always direct, reading doesn't need privileges)
pub fn read_binding(domain: &str) -> Option<String> {
    HostsManager::read_binding(domain)
}

/// Flush DNS using Service if available, otherwise direct
/// On service failure, automatically falls back to direct operation
pub fn flush_dns() -> Result<(), HostsError> {
    #[cfg(windows)]
    {
        if is_service_running() {
            let client = PipeClient::new();
            match client.flush_dns() {
                Ok(()) => return Ok(()),
                Err(e) => {
                    // Service failed - mark unavailable and fall back to direct
                    eprintln!("Service flush_dns failed, falling back to direct: {}", e);
                    mark_service_unavailable();
                    // Fall through to direct operation
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(helper_path) = get_macos_helper_path() {
            match Command::new(&helper_path).args(["flush-dns"]).output() {
                Ok(output) => {
                    if output.status.success() {
                        return Ok(());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("macOS helper flush_dns failed: {}", stderr);
                        // Fall through to direct operation
                    }
                }
                Err(e) => {
                    eprintln!("Failed to execute macOS helper: {}", e);
                    // Fall through to direct operation
                }
            }
        }
    }

    HostsManager::flush_dns()
}

/// Get permission status
/// Returns: (has_permission, is_using_service_or_helper)
pub fn get_permission_status() -> (bool, bool) {
    #[cfg(windows)]
    {
        let service_running = is_service_running();
        if service_running {
            return (true, true);
        }

        // Check if we have direct admin access
        use std::fs::OpenOptions;
        let path = r"C:\Windows\System32\drivers\etc\hosts";
        let has_admin = OpenOptions::new().append(true).open(path).is_ok();
        (has_admin, false)
    }

    #[cfg(target_os = "macos")]
    {
        // Check if macOS helper is available
        if is_macos_helper_available() {
            return (true, true);
        }

        // Check if running as root
        use std::process::Command;
        let output = Command::new("id").arg("-u").output();
        let has_root = match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "0",
            Err(_) => false,
        };
        (has_root, false)
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        // On other Unix systems, check if running as root
        use std::process::Command;
        let output = Command::new("id").arg("-u").output();
        let has_root = match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "0",
            Err(_) => false,
        };
        (has_root, false)
    }
}
