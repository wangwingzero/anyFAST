//! Windows hosts file manager

use fs2::FileExt;
use std::fs;
use std::io::{Read as IoRead, Seek, SeekFrom, Write};
use std::net::IpAddr;
use std::path::Path;
use thiserror::Error;

#[cfg(windows)]
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

#[cfg(not(windows))]
const HOSTS_PATH: &str = "/etc/hosts";

const MARKER: &str = "# AnyRouter";

#[derive(Error, Debug)]
pub enum HostsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Permission denied - run as administrator")]
    PermissionDenied,
    #[error("Invalid IP address: {0}")]
    InvalidIp(String),
    #[error("Invalid domain: {0}")]
    InvalidDomain(String),
}

/// Validate IP address
fn validate_ip(ip: &str) -> Result<(), HostsError> {
    ip.parse::<IpAddr>()
        .map_err(|_| HostsError::InvalidIp(ip.to_string()))?;
    Ok(())
}

/// Validate domain name (no whitespace, control chars, or newlines)
fn validate_domain(domain: &str) -> Result<(), HostsError> {
    if domain.is_empty() {
        return Err(HostsError::InvalidDomain("empty domain".to_string()));
    }
    // Check for invalid characters: whitespace, control chars, newlines
    if domain.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(HostsError::InvalidDomain(format!(
            "contains invalid characters: {}",
            domain
        )));
    }
    // Basic hostname validation: only alphanumeric, hyphens, dots
    if !domain
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '.')
    {
        return Err(HostsError::InvalidDomain(format!(
            "invalid hostname format: {}",
            domain
        )));
    }
    Ok(())
}

/// Binding entry for batch operations
pub struct HostsBinding {
    pub domain: String,
    pub ip: String,
}

pub struct HostsManager;

impl HostsManager {
    /// Read current binding for a domain
    pub fn read_binding(domain: &str) -> Option<String> {
        Self::read_binding_from_path(Path::new(HOSTS_PATH), domain)
    }

    /// Internal: read binding from custom path (for testing)
    fn read_binding_from_path(path: &Path, domain: &str) -> Option<String> {
        let content = fs::read_to_string(path).ok()?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == domain {
                return Some(parts[0].to_string());
            }
        }

        None
    }

    /// Write or update binding in hosts file
    pub fn write_binding(domain: &str, ip: &str) -> Result<(), HostsError> {
        Self::write_binding_to_path(Path::new(HOSTS_PATH), domain, ip)
    }

    /// Internal: write binding to custom path (for testing)
    fn write_binding_to_path(path: &Path, domain: &str, ip: &str) -> Result<(), HostsError> {
        // Validate inputs to prevent injection
        validate_ip(ip)?;
        validate_domain(domain)?;

        // Open file with exclusive lock for atomic read-modify-write
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    HostsError::PermissionDenied
                } else {
                    HostsError::Io(e)
                }
            })?;

        // Acquire exclusive lock (blocks until available)
        file.lock_exclusive().map_err(HostsError::Io)?;

        // Read existing content using the locked file handle (not reopening!)
        let mut raw_content = Vec::new();
        file.read_to_end(&mut raw_content).map_err(HostsError::Io)?;

        let content = if raw_content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            String::from_utf8_lossy(&raw_content[3..]).to_string()
        } else {
            String::from_utf8_lossy(&raw_content).to_string()
        };

        let mut lines: Vec<String> = Vec::new();
        let mut found = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Check if this line is for our domain
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && parts[1] == domain {
                    // Update existing entry
                    lines.push(format!("{}\t{}\t{}", ip, domain, MARKER));
                    found = true;
                    continue;
                }
            }

            lines.push(line.to_string());
        }

        // Add new entry if not found
        if !found {
            // Ensure there's a newline before our entry
            if !lines.is_empty() && !lines.last().unwrap().is_empty() {
                lines.push(String::new());
            }
            lines.push(format!("{}\t{}\t{}", ip, domain, MARKER));
        }

        // Write back (reuse the locked file handle)
        file.set_len(0).map_err(HostsError::Io)?;
        file.seek(SeekFrom::Start(0)).map_err(HostsError::Io)?;

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                writeln!(file)?;
            }
            write!(file, "{}", line)?;
        }

        // Lock is automatically released when file is dropped
        Ok(())
    }

    /// Batch write multiple bindings in a single file operation
    /// More efficient than calling write_binding multiple times
    pub fn write_bindings_batch(bindings: &[HostsBinding]) -> Result<usize, HostsError> {
        Self::write_bindings_batch_to_path(Path::new(HOSTS_PATH), bindings)
    }

    /// Internal: batch write to custom path (for testing)
    fn write_bindings_batch_to_path(path: &Path, bindings: &[HostsBinding]) -> Result<usize, HostsError> {
        if bindings.is_empty() {
            return Ok(0);
        }

        // Validate all inputs first
        for binding in bindings {
            validate_ip(&binding.ip)?;
            validate_domain(&binding.domain)?;
        }

        // Open file with exclusive lock for atomic read-modify-write
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    HostsError::PermissionDenied
                } else {
                    HostsError::Io(e)
                }
            })?;

        // Acquire exclusive lock (blocks until available)
        file.lock_exclusive().map_err(HostsError::Io)?;

        // Read existing content using the locked file handle (not reopening!)
        let mut raw_content = Vec::new();
        file.read_to_end(&mut raw_content).map_err(HostsError::Io)?;

        let content = if raw_content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            String::from_utf8_lossy(&raw_content[3..]).to_string()
        } else {
            String::from_utf8_lossy(&raw_content).to_string()
        };

        // Build a set of domains to update
        let domains_to_update: std::collections::HashSet<&str> =
            bindings.iter().map(|b| b.domain.as_str()).collect();

        let mut lines: Vec<String> = Vec::new();
        let mut updated_count = 0;

        for line in content.lines() {
            let trimmed = line.trim();

            // Check if this line is for one of our domains
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && domains_to_update.contains(parts[1]) {
                    // Skip this line - we'll add updated entries at the end
                    continue;
                }
            }

            lines.push(line.to_string());
        }

        // Ensure there's a newline before our entries
        if !lines.is_empty() && !lines.last().unwrap().is_empty() {
            lines.push(String::new());
        }

        // Add all new/updated bindings
        for binding in bindings {
            lines.push(format!("{}\t{}\t{}", binding.ip, binding.domain, MARKER));
            updated_count += 1;
        }

        // Write back (reuse the locked file handle)
        file.set_len(0).map_err(HostsError::Io)?;
        file.seek(SeekFrom::Start(0)).map_err(HostsError::Io)?;

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                writeln!(file)?;
            }
            write!(file, "{}", line)?;
        }

        // Lock is automatically released when file is dropped
        Ok(updated_count)
    }

    /// Clear binding for a domain
    pub fn clear_binding(domain: &str) -> Result<(), HostsError> {
        Self::clear_binding_from_path(Path::new(HOSTS_PATH), domain)
    }

    /// Internal: clear binding from custom path (for testing)
    fn clear_binding_from_path(path: &Path, domain: &str) -> Result<(), HostsError> {
        let content = fs::read(path)?;
        let content = if content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            String::from_utf8_lossy(&content[3..]).to_string()
        } else {
            String::from_utf8_lossy(&content).to_string()
        };

        let lines: Vec<&str> = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return true;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                !(parts.len() >= 2 && parts[1] == domain)
            })
            .collect();

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)?;

        write!(file, "{}", lines.join("\n"))?;

        Ok(())
    }

    /// Clear multiple bindings in a single file operation
    pub fn clear_bindings_batch(domains: &[&str]) -> Result<usize, HostsError> {
        Self::clear_bindings_batch_from_path(Path::new(HOSTS_PATH), domains)
    }

    /// Internal: clear bindings from custom path (for testing)
    fn clear_bindings_batch_from_path(path: &Path, domains: &[&str]) -> Result<usize, HostsError> {
        if domains.is_empty() {
            return Ok(0);
        }

        let domains_set: std::collections::HashSet<&str> = domains.iter().copied().collect();

        // Open file with exclusive lock for atomic read-modify-write
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    HostsError::PermissionDenied
                } else {
                    HostsError::Io(e)
                }
            })?;

        // Acquire exclusive lock (blocks until available)
        file.lock_exclusive().map_err(HostsError::Io)?;

        // Read existing content using the locked file handle (not reopening!)
        let mut raw_content = Vec::new();
        file.read_to_end(&mut raw_content).map_err(HostsError::Io)?;

        let content = if raw_content.starts_with(&[0xEF, 0xBB, 0xBF]) {
            String::from_utf8_lossy(&raw_content[3..]).to_string()
        } else {
            String::from_utf8_lossy(&raw_content).to_string()
        };

        let mut removed_count = 0;
        let lines: Vec<&str> = content
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    return true;
                }
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 && domains_set.contains(parts[1]) {
                    removed_count += 1;
                    false
                } else {
                    true
                }
            })
            .collect();

        // Write back (reuse the locked file handle)
        file.set_len(0).map_err(HostsError::Io)?;
        file.seek(SeekFrom::Start(0)).map_err(HostsError::Io)?;

        write!(file, "{}", lines.join("\n"))?;

        Ok(removed_count)
    }

    /// Flush DNS cache
    pub fn flush_dns() -> Result<(), HostsError> {
        #[cfg(windows)]
        {
            std::process::Command::new("ipconfig")
                .args(["/flushdns"])
                .output()?;
        }

        #[cfg(not(windows))]
        {
            // macOS
            std::process::Command::new("dscacheutil")
                .args(["-flushcache"])
                .output()
                .ok();
        }

        Ok(())
    }
}

/// Testable version of HostsManager with custom path
#[cfg(test)]
pub struct TestableHostsManager {
    path: std::path::PathBuf,
}

#[cfg(test)]
impl TestableHostsManager {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    pub fn read_binding(&self, domain: &str) -> Option<String> {
        HostsManager::read_binding_from_path(&self.path, domain)
    }

    pub fn write_binding(&self, domain: &str, ip: &str) -> Result<(), HostsError> {
        HostsManager::write_binding_to_path(&self.path, domain, ip)
    }

    pub fn write_bindings_batch(&self, bindings: &[HostsBinding]) -> Result<usize, HostsError> {
        HostsManager::write_bindings_batch_to_path(&self.path, bindings)
    }

    pub fn clear_binding(&self, domain: &str) -> Result<(), HostsError> {
        HostsManager::clear_binding_from_path(&self.path, domain)
    }

    pub fn clear_bindings_batch(&self, domains: &[&str]) -> Result<usize, HostsError> {
        HostsManager::clear_bindings_batch_from_path(&self.path, domains)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_hosts_file(dir: &TempDir, content: &str) -> std::path::PathBuf {
        let path = dir.path().join("hosts");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_read_binding_empty_file() {
        let dir = TempDir::new().unwrap();
        let path = create_hosts_file(&dir, "");
        let manager = TestableHostsManager::new(path);

        assert!(manager.read_binding("test.com").is_none());
    }

    #[test]
    fn test_read_binding_not_found() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n::1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path);

        assert!(manager.read_binding("test.com").is_none());
    }

    #[test]
    fn test_read_binding_found() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n1.2.3.4\ttest.com\t# AnyRouter";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path);

        let ip = manager.read_binding("test.com");
        assert_eq!(ip, Some("1.2.3.4".to_string()));
    }

    #[test]
    fn test_write_binding_new_entry() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.write_binding("test.com", "1.2.3.4").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("1.2.3.4\ttest.com\t# AnyRouter"));
        assert!(result.contains("localhost"));
    }

    #[test]
    fn test_write_binding_update_existing() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n1.1.1.1\ttest.com\t# AnyRouter";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.write_binding("test.com", "2.2.2.2").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("2.2.2.2\ttest.com\t# AnyRouter"));
        assert!(!result.contains("1.1.1.1"));
    }

    #[test]
    fn test_write_bindings_batch() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        let bindings = vec![
            HostsBinding { domain: "test1.com".into(), ip: "1.1.1.1".into() },
            HostsBinding { domain: "test2.com".into(), ip: "2.2.2.2".into() },
        ];

        let count = manager.write_bindings_batch(&bindings).unwrap();
        assert_eq!(count, 2);

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("1.1.1.1\ttest1.com\t# AnyRouter"));
        assert!(result.contains("2.2.2.2\ttest2.com\t# AnyRouter"));
    }

    #[test]
    fn test_write_bindings_batch_empty() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path);

        let count = manager.write_bindings_batch(&[]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_clear_binding() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n1.2.3.4\ttest.com\t# AnyRouter";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.clear_binding("test.com").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(!result.contains("test.com"));
        assert!(result.contains("localhost"));
    }

    #[test]
    fn test_clear_bindings_batch() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n1.1.1.1\ttest1.com\n2.2.2.2\ttest2.com";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        let count = manager.clear_bindings_batch(&["test1.com", "test2.com"]).unwrap();
        assert_eq!(count, 2);

        let result = fs::read_to_string(&path).unwrap();
        assert!(!result.contains("test1.com"));
        assert!(!result.contains("test2.com"));
        assert!(result.contains("localhost"));
    }

    #[test]
    fn test_clear_bindings_batch_empty() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path);

        let count = manager.clear_bindings_batch(&[]).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_bom_handling() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("hosts");

        // Write file with UTF-8 BOM
        let bom = [0xEF, 0xBB, 0xBF];
        let content = "127.0.0.1\tlocalhost\n1.2.3.4\ttest.com";
        let mut data = bom.to_vec();
        data.extend(content.as_bytes());
        fs::write(&path, data).unwrap();

        let manager = TestableHostsManager::new(path.clone());

        // Reading should work with BOM
        let ip = manager.read_binding("test.com");
        assert_eq!(ip, Some("1.2.3.4".to_string()));

        // Writing should work with BOM
        manager.write_binding("test2.com", "5.6.7.8").unwrap();
        let ip2 = manager.read_binding("test2.com");
        assert_eq!(ip2, Some("5.6.7.8".to_string()));
    }

    #[test]
    fn test_preserves_comments() {
        let dir = TempDir::new().unwrap();
        let content = "# This is a comment\n127.0.0.1\tlocalhost\n# Another comment";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.write_binding("test.com", "1.2.3.4").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("# This is a comment"));
        assert!(result.contains("# Another comment"));
    }

    #[test]
    fn test_marker_identification() {
        // Test that MARKER constant is correct
        assert_eq!(MARKER, "# AnyRouter");
    }
}
