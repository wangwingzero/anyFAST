//! Named Pipe client for communicating with the anyFAST hosts service
//!
//! This module provides a client that connects to the privileged service
//! to perform hosts file operations without requiring admin privileges.

use crate::service::rpc::*;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileA, ReadFile, WriteFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE,
    OPEN_EXISTING,
};
use windows::Win32::System::Pipes::{
    SetNamedPipeHandleState, WaitNamedPipeA, PIPE_READMODE_MESSAGE,
};

/// Pipe name (must match server)
const PIPE_NAME: &str = r"\\.\pipe\anyfast-hosts-service";

/// Connection timeout in milliseconds
const CONNECT_TIMEOUT_MS: u32 = 5000;

/// Buffer size for communication
const BUFFER_SIZE: usize = 65536;

#[derive(Error, Debug)]
pub enum PipeClientError {
    #[error("Service not running or pipe not available")]
    ServiceNotRunning,
    #[error("Connection timeout")]
    ConnectionTimeout,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("RPC error: {code} - {message}")]
    Rpc { code: i32, message: String },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid response")]
    InvalidResponse,
}

/// Client for communicating with the anyFAST hosts service
pub struct PipeClient {
    request_id: AtomicU64,
}

impl PipeClient {
    pub fn new() -> Self {
        Self {
            request_id: AtomicU64::new(1),
        }
    }

    /// Generate a unique request ID
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Connect to the service pipe
    fn connect(&self) -> Result<HANDLE, PipeClientError> {
        let pipe_name = format!("{}\0", PIPE_NAME);

        // Wait for the pipe to become available
        let available = unsafe {
            WaitNamedPipeA(PCSTR::from_raw(pipe_name.as_ptr()), CONNECT_TIMEOUT_MS)
        };

        if available.is_err() {
            let err = std::io::Error::last_os_error();
            // ERROR_SEM_TIMEOUT (121) means timeout waiting for pipe
            if err.raw_os_error() == Some(121) {
                return Err(PipeClientError::ConnectionTimeout);
            }
            return Err(PipeClientError::ServiceNotRunning);
        }

        // Open the pipe
        let handle = unsafe {
            CreateFileA(
                PCSTR::from_raw(pipe_name.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_GENERIC_READ.0
                    | windows::Win32::Storage::FileSystem::FILE_GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        let handle = match handle {
            Ok(h) => h,
            Err(_) => return Err(PipeClientError::ServiceNotRunning),
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(PipeClientError::ServiceNotRunning);
        }

        // Set message mode
        let mode = PIPE_READMODE_MESSAGE;
        let result = unsafe {
            SetNamedPipeHandleState(handle, Some(&mode), None, None)
        };

        if result.is_err() {
            unsafe { CloseHandle(handle) }.ok();
            return Err(PipeClientError::Io(std::io::Error::last_os_error()));
        }

        Ok(handle)
    }

    /// Send a request and receive response
    fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, PipeClientError> {
        let handle = self.connect()?;

        // Ensure handle is closed on exit
        let _guard = HandleGuard(handle);

        let request_id = self.next_id();
        let request = RpcRequest::new(request_id, method, params);
        let request_json = serde_json::to_vec(&request)?;

        // Write request
        let mut bytes_written: u32 = 0;
        let write_result = unsafe {
            WriteFile(handle, Some(&request_json), Some(&mut bytes_written), None)
        };

        if write_result.is_err() {
            return Err(PipeClientError::Io(std::io::Error::last_os_error()));
        }

        // Read response
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut bytes_read: u32 = 0;
        let read_result = unsafe {
            ReadFile(handle, Some(&mut buffer), Some(&mut bytes_read), None)
        };

        if read_result.is_err() {
            return Err(PipeClientError::Io(std::io::Error::last_os_error()));
        }

        // Parse response
        let response: RpcResponse = serde_json::from_slice(&buffer[..bytes_read as usize])?;

        // Validate response matches our request
        if response.jsonrpc != "2.0" {
            return Err(PipeClientError::InvalidResponse);
        }
        if response.id != request_id {
            return Err(PipeClientError::InvalidResponse);
        }

        // Check for RPC error
        if let Some(error) = response.error {
            return Err(PipeClientError::Rpc {
                code: error.code,
                message: error.message,
            });
        }

        response.result.ok_or(PipeClientError::InvalidResponse)
    }

    // ============ Public API ============

    /// Check if the service is running
    pub fn is_service_running(&self) -> bool {
        self.ping().is_ok()
    }

    /// Ping the service
    pub fn ping(&self) -> Result<PingResult, PipeClientError> {
        let result = self.call(methods::PING, serde_json::Value::Null)?;
        Ok(serde_json::from_value(result)?)
    }

    /// Write a single binding
    pub fn write_binding(&self, domain: &str, ip: &str) -> Result<(), PipeClientError> {
        let params = WriteBindingParams {
            domain: domain.to_string(),
            ip: ip.to_string(),
        };
        let result = self.call(methods::WRITE_BINDING, serde_json::to_value(params)?)?;
        let success: SuccessResult = serde_json::from_value(result)?;
        if success.success {
            Ok(())
        } else {
            Err(PipeClientError::InvalidResponse)
        }
    }

    /// Write multiple bindings in batch
    pub fn write_bindings_batch(
        &self,
        bindings: &[(String, String)],
    ) -> Result<u32, PipeClientError> {
        let params = WriteBindingsBatchParams {
            bindings: bindings
                .iter()
                .map(|(domain, ip)| BindingEntry {
                    domain: domain.clone(),
                    ip: ip.clone(),
                })
                .collect(),
        };
        let result = self.call(methods::WRITE_BINDINGS_BATCH, serde_json::to_value(params)?)?;
        let count: CountResult = serde_json::from_value(result)?;
        Ok(count.count)
    }

    /// Clear a single binding
    pub fn clear_binding(&self, domain: &str) -> Result<(), PipeClientError> {
        let params = ClearBindingParams {
            domain: domain.to_string(),
        };
        let result = self.call(methods::CLEAR_BINDING, serde_json::to_value(params)?)?;
        let success: SuccessResult = serde_json::from_value(result)?;
        if success.success {
            Ok(())
        } else {
            Err(PipeClientError::InvalidResponse)
        }
    }

    /// Clear multiple bindings in batch
    pub fn clear_bindings_batch(&self, domains: &[String]) -> Result<u32, PipeClientError> {
        let params = ClearBindingsBatchParams {
            domains: domains.to_vec(),
        };
        let result = self.call(methods::CLEAR_BINDINGS_BATCH, serde_json::to_value(params)?)?;
        let count: CountResult = serde_json::from_value(result)?;
        Ok(count.count)
    }

    /// Read a binding
    pub fn read_binding(&self, domain: &str) -> Result<Option<String>, PipeClientError> {
        let params = ReadBindingParams {
            domain: domain.to_string(),
        };
        let result = self.call(methods::READ_BINDING, serde_json::to_value(params)?)?;
        let binding: ReadBindingResult = serde_json::from_value(result)?;
        Ok(binding.ip)
    }

    /// Get all bindings
    pub fn get_all_bindings(&self) -> Result<Vec<(String, String)>, PipeClientError> {
        let result = self.call(methods::GET_ALL_BINDINGS, serde_json::Value::Null)?;
        let bindings: AllBindingsResult = serde_json::from_value(result)?;
        Ok(bindings
            .bindings
            .into_iter()
            .map(|b| (b.domain, b.ip))
            .collect())
    }

    /// Flush DNS cache
    pub fn flush_dns(&self) -> Result<(), PipeClientError> {
        let result = self.call(methods::FLUSH_DNS, serde_json::Value::Null)?;
        let success: SuccessResult = serde_json::from_value(result)?;
        if success.success {
            Ok(())
        } else {
            Err(PipeClientError::InvalidResponse)
        }
    }
}

impl Default for PipeClient {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard to ensure handle is closed
struct HandleGuard(HANDLE);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) }.ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = PipeClient::new();
        // Just test that we can create a client
        assert!(client.next_id() > 0);
    }
}
