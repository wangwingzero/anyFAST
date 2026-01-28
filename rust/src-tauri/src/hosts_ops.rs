//! Hosts operations with Service fallback
//!
//! This module provides a unified interface for hosts file operations
//! that automatically uses the Windows Service when available, or falls
//! back to direct file operations when running with admin privileges.

use crate::hosts_manager::{HostsBinding, HostsError, HostsManager};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[cfg(windows)]
use crate::client::PipeClient;

/// Cached state of whether the service is running
static SERVICE_AVAILABLE: OnceLock<AtomicBool> = OnceLock::new();

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

    HostsManager::write_bindings_batch(bindings)
}

/// Clear a binding using Service if available, otherwise direct
/// On service failure, automatically falls back to direct operation
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

    HostsManager::clear_bindings_batch(domains)
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

    HostsManager::flush_dns()
}

/// Get permission status
/// Returns: (has_permission, is_using_service)
pub fn get_permission_status() -> (bool, bool) {
    let service_running = is_service_running();
    if service_running {
        return (true, true);
    }

    // Check if we have direct admin access
    #[cfg(windows)]
    {
        use std::fs::OpenOptions;
        let path = r"C:\Windows\System32\drivers\etc\hosts";
        let has_admin = OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .is_ok();
        (has_admin, false)
    }

    #[cfg(not(windows))]
    {
        // On Unix, check if running as root using nix or std
        use std::process::Command;
        let output = Command::new("id").arg("-u").output();
        let has_root = match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "0",
            Err(_) => false,
        };
        (has_root, false)
    }
}
