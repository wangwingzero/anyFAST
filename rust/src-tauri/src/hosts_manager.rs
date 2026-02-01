//! Windows hosts file manager
//!
//! Features:
//! - Block-based management with BEGIN/END markers
//! - Atomic file writes using temp file + fsync + rename
//! - Exclusive file locking for concurrent access safety
//! - UTF-8 BOM handling

use fs2::FileExt;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read as IoRead, Write};
use std::net::IpAddr;
use std::path::Path;
use thiserror::Error;

#[cfg(windows)]
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

#[cfg(not(windows))]
const HOSTS_PATH: &str = "/etc/hosts";

// Block markers for identifying anyFAST-managed entries
const MARKER_BEGIN: &str = "# BEGIN anyFAST";
const MARKER_END: &str = "# END anyFAST";
const MARKER_LINE: &str = "# anyFAST";

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
    // Basic hostname validation: only alphanumeric, hyphens, dots, underscores
    if !domain
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '_')
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

/// Internal structure to hold parsed hosts file content
struct ParsedHosts {
    /// Lines before the anyFAST block
    before_block: Vec<String>,
    /// Lines after the anyFAST block
    after_block: Vec<String>,
    /// Current anyFAST bindings (domain -> ip)
    anyrouter_bindings: std::collections::HashMap<String, String>,
}

impl ParsedHosts {
    fn parse(content: &str) -> Self {
        let mut before_block = Vec::new();
        let mut after_block = Vec::new();
        let mut anyrouter_bindings = std::collections::HashMap::new();

        let mut in_block = false;
        let mut found_block = false;
        // Track lines seen while in_block in case END marker is missing
        let mut unclosed_block_lines = Vec::new();

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
                // Parse binding inside the block
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        anyrouter_bindings.insert(parts[1].to_string(), parts[0].to_string());
                    }
                }
                // Track raw lines in case block is unclosed
                unclosed_block_lines.push(line.to_string());
            } else if found_block {
                after_block.push(line.to_string());
            } else {
                // Also check for legacy line-level markers (for backward compatibility)
                if trimmed.contains(MARKER_LINE) && !trimmed.is_empty() && !trimmed.starts_with('#')
                {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        anyrouter_bindings.insert(parts[1].to_string(), parts[0].to_string());
                    }
                } else {
                    before_block.push(line.to_string());
                }
            }
        }

        // Handle unclosed block: if END marker is missing, preserve trailing content
        // that wasn't valid bindings (e.g., user comments added after BEGIN)
        if in_block && !unclosed_block_lines.is_empty() {
            // Lines that weren't parsed as bindings should be preserved in after_block
            // This prevents data loss when END marker is accidentally deleted
            for line in unclosed_block_lines {
                let trimmed = line.trim();
                // Skip lines that were already parsed as bindings
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    after_block.push(line);
                } else {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    // If it doesn't look like a binding, preserve it
                    if parts.len() < 2 || parts[0].parse::<IpAddr>().is_err() {
                        after_block.push(line);
                    }
                }
            }
        }

        ParsedHosts {
            before_block,
            after_block,
            anyrouter_bindings,
        }
    }

    fn render(&self) -> String {
        let mut lines = self.before_block.clone();

        // Add anyFAST block if there are bindings
        if !self.anyrouter_bindings.is_empty() {
            // Ensure there's a blank line before the block
            if !lines.is_empty() && !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
                lines.push(String::new());
            }

            lines.push(MARKER_BEGIN.to_string());

            // Sort bindings by domain for consistent output
            let mut sorted_bindings: Vec<_> = self.anyrouter_bindings.iter().collect();
            sorted_bindings.sort_by_key(|(domain, _)| *domain);

            for (domain, ip) in sorted_bindings {
                lines.push(format!("{}\t{}\t{}", ip, domain, MARKER_LINE));
            }

            lines.push(MARKER_END.to_string());
        }

        // Add lines after the block
        lines.extend(self.after_block.clone());

        // Join with newlines
        lines.join("\n")
    }
}

/// Read file content handling UTF-8 BOM
fn read_hosts_content(file: &mut File) -> Result<String, HostsError> {
    let mut raw_content = Vec::new();
    file.read_to_end(&mut raw_content).map_err(HostsError::Io)?;

    let content = if raw_content.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&raw_content[3..]).to_string()
    } else {
        String::from_utf8_lossy(&raw_content).to_string()
    };

    Ok(content)
}

/// Atomic write: write to temp file, fsync, then rename
fn atomic_write(path: &Path, content: &str) -> Result<(), HostsError> {
    // Create temp file in the same directory (required for atomic rename)
    let parent = path.parent().unwrap_or(Path::new("."));
    let temp_path = parent.join(format!(".hosts.tmp.{}", std::process::id()));

    // Write to temp file
    {
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    HostsError::PermissionDenied
                } else {
                    HostsError::Io(e)
                }
            })?;

        temp_file.write_all(content.as_bytes())?;
        temp_file.flush()?;
        temp_file.sync_all()?; // fsync to ensure data is on disk
    }

    // Atomic rename
    fs::rename(&temp_path, path).map_err(|e| {
        // Clean up temp file on failure
        let _ = fs::remove_file(&temp_path);
        HostsError::Io(e)
    })?;

    Ok(())
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
        let parsed = ParsedHosts::parse(&content);

        // First check anyFAST bindings
        if let Some(ip) = parsed.anyrouter_bindings.get(domain) {
            return Some(ip.clone());
        }

        // Fall back to checking all lines (for non-anyFAST entries)
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
        let mut file = OpenOptions::new()
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

        // Read and parse existing content
        let content = read_hosts_content(&mut file)?;
        let mut parsed = ParsedHosts::parse(&content);

        // Update or add binding
        parsed
            .anyrouter_bindings
            .insert(domain.to_string(), ip.to_string());

        // Generate new content
        let new_content = parsed.render();

        // Atomic write
        atomic_write(path, &new_content)?;

        // Lock is automatically released when file is dropped
        Ok(())
    }

    /// Batch write multiple bindings in a single file operation
    /// More efficient than calling write_binding multiple times
    pub fn write_bindings_batch(bindings: &[HostsBinding]) -> Result<usize, HostsError> {
        Self::write_bindings_batch_to_path(Path::new(HOSTS_PATH), bindings)
    }

    /// Internal: batch write to custom path (for testing)
    fn write_bindings_batch_to_path(
        path: &Path,
        bindings: &[HostsBinding],
    ) -> Result<usize, HostsError> {
        if bindings.is_empty() {
            return Ok(0);
        }

        // Validate all inputs first
        for binding in bindings {
            validate_ip(&binding.ip)?;
            validate_domain(&binding.domain)?;
        }

        // Open file with exclusive lock for atomic read-modify-write
        let mut file = OpenOptions::new()
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

        // Read and parse existing content
        let content = read_hosts_content(&mut file)?;
        let mut parsed = ParsedHosts::parse(&content);

        // Update bindings
        let mut updated_count = 0;
        for binding in bindings {
            parsed
                .anyrouter_bindings
                .insert(binding.domain.clone(), binding.ip.clone());
            updated_count += 1;
        }

        // Generate new content
        let new_content = parsed.render();

        // Atomic write
        atomic_write(path, &new_content)?;

        // Lock is automatically released when file is dropped
        Ok(updated_count)
    }

    /// Clear binding for a domain
    #[allow(dead_code)]
    pub fn clear_binding(domain: &str) -> Result<(), HostsError> {
        Self::clear_binding_from_path(Path::new(HOSTS_PATH), domain)
    }

    /// Internal: clear binding from custom path (for testing)
    /// Now uses file locking for safety
    #[allow(dead_code)]
    fn clear_binding_from_path(path: &Path, domain: &str) -> Result<(), HostsError> {
        // Open file with exclusive lock for atomic read-modify-write
        let mut file = OpenOptions::new()
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

        // Read and parse existing content
        let content = read_hosts_content(&mut file)?;
        let mut parsed = ParsedHosts::parse(&content);

        // Remove binding
        parsed.anyrouter_bindings.remove(domain);

        // Generate new content
        let new_content = parsed.render();

        // Atomic write
        atomic_write(path, &new_content)?;

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

        let domains_set: HashSet<&str> = domains.iter().copied().collect();

        // Open file with exclusive lock for atomic read-modify-write
        let mut file = OpenOptions::new()
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

        // Read and parse existing content
        let content = read_hosts_content(&mut file)?;
        let mut parsed = ParsedHosts::parse(&content);

        // Remove bindings and count
        let mut removed_count = 0;
        for domain in &domains_set {
            if parsed.anyrouter_bindings.remove(*domain).is_some() {
                removed_count += 1;
            }
        }

        // Generate new content
        let new_content = parsed.render();

        // Atomic write
        atomic_write(path, &new_content)?;

        Ok(removed_count)
    }

    /// Clear ALL anyFAST-managed bindings from hosts file
    /// This removes the entire anyFAST block regardless of current config
    pub fn clear_all_anyfast_bindings() -> Result<usize, HostsError> {
        Self::clear_all_anyfast_bindings_from_path(Path::new(HOSTS_PATH))
    }

    /// Internal: clear all anyFAST bindings from custom path (for testing)
    fn clear_all_anyfast_bindings_from_path(path: &Path) -> Result<usize, HostsError> {
        // Open file with exclusive lock for atomic read-modify-write
        let mut file = OpenOptions::new()
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

        // Read and parse existing content
        let content = read_hosts_content(&mut file)?;
        let mut parsed = ParsedHosts::parse(&content);

        // Count and clear all bindings
        let removed_count = parsed.anyrouter_bindings.len();
        parsed.anyrouter_bindings.clear();

        // Generate new content (will not include anyFAST block since bindings is empty)
        let new_content = parsed.render();

        // Atomic write
        atomic_write(path, &new_content)?;

        Ok(removed_count)
    }

    /// Flush DNS cache
    pub fn flush_dns() -> Result<(), HostsError> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            
            // Use absolute path to prevent PATH injection attacks
            // Use CREATE_NO_WINDOW to hide the console window flash
            std::process::Command::new(r"C:\Windows\System32\ipconfig.exe")
                .args(["/flushdns"])
                .creation_flags(CREATE_NO_WINDOW)
                .output()?;
        }

        #[cfg(not(windows))]
        {
            // macOS - use absolute path
            std::process::Command::new("/usr/bin/dscacheutil")
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
    fn test_read_binding_found_in_block() {
        let dir = TempDir::new().unwrap();
        let content =
            "127.0.0.1\tlocalhost\n# BEGIN anyFAST\n1.2.3.4\ttest.com\t# anyFAST\n# END anyFAST";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path);

        let ip = manager.read_binding("test.com");
        assert_eq!(ip, Some("1.2.3.4".to_string()));
    }

    #[test]
    fn test_read_binding_legacy_format() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n1.2.3.4\ttest.com\t# anyFAST";
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
        assert!(result.contains("1.2.3.4\ttest.com"));
        assert!(result.contains("localhost"));
        assert!(result.contains(MARKER_BEGIN));
        assert!(result.contains(MARKER_END));
        assert!(result.contains(MARKER_LINE));
    }

    #[test]
    fn test_write_binding_update_existing() {
        let dir = TempDir::new().unwrap();
        let content =
            "127.0.0.1\tlocalhost\n# BEGIN anyFAST\n1.1.1.1\ttest.com\t# anyFAST\n# END anyFAST";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.write_binding("test.com", "2.2.2.2").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("2.2.2.2\ttest.com"));
        assert!(!result.contains("1.1.1.1"));
    }

    #[test]
    fn test_write_bindings_batch() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        let bindings = vec![
            HostsBinding {
                domain: "test1.com".into(),
                ip: "1.1.1.1".into(),
            },
            HostsBinding {
                domain: "test2.com".into(),
                ip: "2.2.2.2".into(),
            },
        ];

        let count = manager.write_bindings_batch(&bindings).unwrap();
        assert_eq!(count, 2);

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("1.1.1.1\ttest1.com"));
        assert!(result.contains("2.2.2.2\ttest2.com"));
        assert!(result.contains(MARKER_BEGIN));
        assert!(result.contains(MARKER_END));
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
        let content =
            "127.0.0.1\tlocalhost\n# BEGIN anyFAST\n1.2.3.4\ttest.com\t# anyFAST\n# END anyFAST";
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
        let content = "127.0.0.1\tlocalhost\n# BEGIN anyFAST\n1.1.1.1\ttest1.com\t# anyFAST\n2.2.2.2\ttest2.com\t# anyFAST\n# END anyFAST";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        let count = manager
            .clear_bindings_batch(&["test1.com", "test2.com"])
            .unwrap();
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
    fn test_marker_block_format() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.write_binding("example.com", "1.2.3.4").unwrap();
        manager.write_binding("test.com", "5.6.7.8").unwrap();

        let result = fs::read_to_string(&path).unwrap();

        // Verify block structure
        assert!(result.contains(MARKER_BEGIN));
        assert!(result.contains(MARKER_END));

        // Verify bindings are sorted alphabetically
        let begin_pos = result.find(MARKER_BEGIN).unwrap();
        let end_pos = result.find(MARKER_END).unwrap();
        let example_pos = result.find("example.com").unwrap();
        let test_pos = result.find("test.com").unwrap();

        assert!(begin_pos < example_pos);
        assert!(example_pos < test_pos);
        assert!(test_pos < end_pos);
    }

    #[test]
    fn test_preserves_non_anyrouter_entries() {
        let dir = TempDir::new().unwrap();
        let content = "127.0.0.1\tlocalhost\n192.168.1.1\tmyserver.local";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        manager.write_binding("test.com", "1.2.3.4").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        // Non-anyFAST entries should be preserved
        assert!(result.contains("127.0.0.1\tlocalhost"));
        assert!(result.contains("192.168.1.1\tmyserver.local"));
    }

    #[test]
    fn test_unclosed_block_preserves_content() {
        let dir = TempDir::new().unwrap();
        // Missing END marker - content after BEGIN should not be lost
        let content = "127.0.0.1\tlocalhost\n# BEGIN anyFAST\n1.2.3.4\ttest.com\t# anyFAST\n# User added comment\n192.168.1.1\tmyserver.local";
        let path = create_hosts_file(&dir, content);
        let manager = TestableHostsManager::new(path.clone());

        // Reading should still work
        let ip = manager.read_binding("test.com");
        assert_eq!(ip, Some("1.2.3.4".to_string()));

        // Writing should preserve non-binding content
        manager.write_binding("new.com", "5.6.7.8").unwrap();

        let result = fs::read_to_string(&path).unwrap();
        assert!(result.contains("127.0.0.1\tlocalhost"));
        // The user comment should be preserved
        assert!(result.contains("# User added comment"));
        // New binding should be added
        assert!(result.contains("5.6.7.8\tnew.com"));
        // Block should now be properly closed
        assert!(result.contains(MARKER_END));
    }
}
