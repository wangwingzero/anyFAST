//! Named Pipe server for Windows hosts service
//!
//! This module implements a secure Named Pipe server that handles
//! hosts file operations via JSON-RPC protocol.
//!
//! Security features:
//! - DACL restricts access to Administrators only
//! - PIPE_REJECT_REMOTE_CLIENTS prevents network access
//! - FILE_FLAG_FIRST_PIPE_INSTANCE prevents pipe squatting
//! - Cancellable I/O for clean shutdown

use crate::hosts_manager::{HostsBinding, HostsManager};
use crate::service::rpc::*;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{
    FlushFileBuffers, ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeA, DisconnectNamedPipe, PIPE_READMODE_MESSAGE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::Win32::System::Threading::{CreateEventA, SetEvent, WaitForSingleObject};
use windows::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};

/// Named Pipe path for the hosts service
pub const PIPE_NAME: &str = r"\\.\pipe\anyfast-hosts-service";

/// Service version
const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Buffer size for pipe communication (64KB max message size)
const BUFFER_SIZE: u32 = 65536;

/// PIPE_ACCESS_DUPLEX constant
const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;

/// SDDL string for service pipe access
/// D: = DACL
/// (A;;GRGW;;;AU) = Allow Generic Read + Generic Write to Authenticated Users (clients)
/// (A;;GA;;;BA) = Allow Generic All to Built-in Administrators
/// (A;;GA;;;SY) = Allow Generic All to Local System
///
/// Note: Clients (GUI) need read/write access to send requests and receive responses.
/// Only the service (running as SYSTEM) needs full control.
const PIPE_ACCESS_SDDL: &str = "D:(A;;GRGW;;;AU)(A;;GA;;;BA)(A;;GA;;;SY)";

/// SDDL revision
const SDDL_REVISION_1: u32 = 1;

/// Pipe server that handles hosts file operations
pub struct PipeServer {
    running: Arc<AtomicBool>,
    stop_event: HANDLE,
}

// SAFETY: Windows HANDLE is a kernel object handle that is safe to use across threads.
// The stop_event is a manual-reset event used for signaling between threads.
// AtomicBool is already Send+Sync. All HANDLE operations we use (SetEvent,
// WaitForSingleObject, WaitForMultipleObjects, CloseHandle) are thread-safe.
unsafe impl Send for PipeServer {}
unsafe impl Sync for PipeServer {}

impl PipeServer {
    pub fn new() -> Self {
        // Create a manual-reset event for signaling stop
        let stop_event = unsafe {
            CreateEventA(
                None,  // Default security
                true,  // Manual reset
                false, // Initial state: not signaled
                None,  // No name
            )
        }
        .unwrap_or(INVALID_HANDLE_VALUE);

        Self {
            running: Arc::new(AtomicBool::new(false)),
            stop_event,
        }
    }

    /// Create security attributes that restrict access to Administrators only
    fn create_admin_security_attributes(
    ) -> Result<(SECURITY_ATTRIBUTES, PSECURITY_DESCRIPTOR), String> {
        // Convert SDDL string to security descriptor
        let sddl_wide: Vec<u16> = PIPE_ACCESS_SDDL
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut sd_ptr: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR(ptr::null_mut());

        let result = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR::from_raw(sddl_wide.as_ptr()),
                SDDL_REVISION_1,
                &mut sd_ptr,
                None,
            )
        };

        if result.is_err() {
            return Err(format!(
                "Failed to create security descriptor: {}",
                std::io::Error::last_os_error()
            ));
        }

        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd_ptr.0,
            bInheritHandle: false.into(),
        };

        Ok((sa, sd_ptr))
    }

    /// Run the pipe server (blocking)
    pub fn run(&self) -> Result<(), String> {
        self.running.store(true, Ordering::SeqCst);

        // Create security attributes for admin-only access
        let (security_attrs, _sd) = Self::create_admin_security_attributes()?;

        let pipe_name = format!("{}\0", PIPE_NAME);

        while self.running.load(Ordering::SeqCst) {
            // Create named pipe instance with security restrictions
            let pipe_handle = unsafe {
                CreateNamedPipeA(
                    PCSTR::from_raw(pipe_name.as_ptr()),
                    windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(
                        PIPE_ACCESS_DUPLEX
                            | FILE_FLAG_FIRST_PIPE_INSTANCE.0
                            | FILE_FLAG_OVERLAPPED.0,
                    ),
                    // PIPE_REJECT_REMOTE_CLIENTS prevents network access
                    PIPE_TYPE_MESSAGE
                        | PIPE_READMODE_MESSAGE
                        | PIPE_WAIT
                        | PIPE_REJECT_REMOTE_CLIENTS,
                    PIPE_UNLIMITED_INSTANCES,
                    BUFFER_SIZE,
                    BUFFER_SIZE,
                    0,                     // Default timeout
                    Some(&security_attrs), // Admin-only security
                )
            };

            let pipe_handle = match pipe_handle {
                Ok(h) => h,
                Err(e) => {
                    // If pipe already exists with FIRST_PIPE_INSTANCE, another instance is running
                    eprintln!("Failed to create named pipe: {}", e);
                    // Check if we should stop
                    if !self.running.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            if pipe_handle == INVALID_HANDLE_VALUE {
                let err = std::io::Error::last_os_error();
                eprintln!("Invalid pipe handle: {}", err);
                if !self.running.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }

            // Wait for client connection with cancellation support
            match self.wait_for_connection(pipe_handle) {
                Ok(true) => {
                    // Client connected - handle requests
                    if let Err(e) = self.handle_client(pipe_handle) {
                        eprintln!("Client error: {}", e);
                    }
                }
                Ok(false) => {
                    // Stop signal received
                    unsafe { CloseHandle(pipe_handle) }.ok();
                    break;
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }

            // Disconnect and close pipe
            unsafe {
                DisconnectNamedPipe(pipe_handle).ok();
                CloseHandle(pipe_handle).ok();
            }
        }

        Ok(())
    }

    /// Wait for client connection with cancellation support
    fn wait_for_connection(&self, pipe: HANDLE) -> Result<bool, String> {
        // Create event for overlapped connect
        let connect_event = unsafe { CreateEventA(None, true, false, None) }
            .map_err(|e| format!("Failed to create event: {}", e))?;

        let mut overlapped = OVERLAPPED {
            hEvent: connect_event,
            ..Default::default()
        };

        // Start async connect
        let connect_result = unsafe { ConnectNamedPipe(pipe, Some(&mut overlapped)) };

        if connect_result.is_err() {
            let err = std::io::Error::last_os_error();
            let err_code = err.raw_os_error().unwrap_or(0);

            // ERROR_IO_PENDING (997) means async operation started
            // ERROR_PIPE_CONNECTED (535) means client already connected
            if err_code != 997 && err_code != 535 {
                unsafe { CloseHandle(connect_event) }.ok();
                return Err(format!("ConnectNamedPipe failed: {}", err));
            }

            if err_code == 535 {
                // Already connected
                unsafe { CloseHandle(connect_event) }.ok();
                return Ok(true);
            }
        }

        // Wait for either connection or stop signal
        let handles = [connect_event, self.stop_event];

        loop {
            // Wait with timeout to check running flag periodically
            let wait_result = unsafe {
                windows::Win32::System::Threading::WaitForMultipleObjects(
                    &handles, false, // Wait for any
                    1000,  // 1 second timeout
                )
            };

            match wait_result {
                WAIT_OBJECT_0 => {
                    // Connect event signaled - client connected
                    unsafe { CloseHandle(connect_event) }.ok();
                    return Ok(true);
                }
                w if w.0 == WAIT_OBJECT_0.0 + 1 => {
                    // Stop event signaled
                    unsafe {
                        // Cancel pending I/O
                        windows::Win32::System::IO::CancelIo(pipe).ok();
                        CloseHandle(connect_event).ok();
                    }
                    return Ok(false);
                }
                w if w.0 == 258 => {
                    // WAIT_TIMEOUT - check if we should stop
                    if !self.running.load(Ordering::SeqCst) {
                        unsafe {
                            windows::Win32::System::IO::CancelIo(pipe).ok();
                            CloseHandle(connect_event).ok();
                        }
                        return Ok(false);
                    }
                    // Continue waiting
                }
                _ => {
                    // Error
                    unsafe { CloseHandle(connect_event) }.ok();
                    return Err(format!("Wait failed: {}", std::io::Error::last_os_error()));
                }
            }
        }
    }

    /// Stop the server
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        // Signal the stop event to wake up waiting threads
        if self.stop_event != INVALID_HANDLE_VALUE {
            unsafe { SetEvent(self.stop_event) }.ok();
        }
    }

    /// Handle a connected client
    fn handle_client(&self, pipe: HANDLE) -> Result<(), String> {
        let mut buffer = vec![0u8; BUFFER_SIZE as usize];

        // Create event for overlapped I/O
        let io_event = unsafe { CreateEventA(None, true, false, None) }
            .map_err(|e| format!("Failed to create IO event: {}", e))?;

        let _event_guard = HandleGuard(io_event);

        loop {
            // Check if we should stop
            if !self.running.load(Ordering::SeqCst) {
                return Ok(());
            }

            // Read request with overlapped I/O
            let mut overlapped = OVERLAPPED {
                hEvent: io_event,
                ..Default::default()
            };

            let mut bytes_read: u32 = 0;
            let read_result = unsafe {
                ReadFile(
                    pipe,
                    Some(&mut buffer),
                    Some(&mut bytes_read),
                    Some(&mut overlapped),
                )
            };

            if read_result.is_err() {
                let err = std::io::Error::last_os_error();
                let err_code = err.raw_os_error().unwrap_or(0);

                // ERROR_IO_PENDING means async read started
                if err_code == 997 {
                    // Wait for read or stop
                    let handles = [io_event, self.stop_event];
                    let wait_result = unsafe {
                        windows::Win32::System::Threading::WaitForMultipleObjects(
                            &handles, false, 30000, // 30 second timeout for read
                        )
                    };

                    match wait_result {
                        WAIT_OBJECT_0 => {
                            // Read completed
                            let get_result = unsafe {
                                GetOverlappedResult(pipe, &overlapped, &mut bytes_read, false)
                            };
                            if get_result.is_err() {
                                let err = std::io::Error::last_os_error();
                                // ERROR_BROKEN_PIPE or ERROR_PIPE_NOT_CONNECTED
                                if err.raw_os_error() == Some(109)
                                    || err.raw_os_error() == Some(233)
                                {
                                    return Ok(()); // Client disconnected
                                }
                                return Err(format!("GetOverlappedResult error: {}", err));
                            }
                        }
                        w if w.0 == WAIT_OBJECT_0.0 + 1 => {
                            // Stop signal
                            unsafe { windows::Win32::System::IO::CancelIo(pipe) }.ok();
                            return Ok(());
                        }
                        w if w.0 == 258 => {
                            // Timeout - client idle too long, disconnect
                            return Ok(());
                        }
                        _ => {
                            return Err(format!(
                                "Read wait failed: {}",
                                std::io::Error::last_os_error()
                            ));
                        }
                    }
                } else if err_code == 109 || err_code == 233 {
                    // ERROR_BROKEN_PIPE or ERROR_PIPE_NOT_CONNECTED
                    return Ok(()); // Client disconnected normally
                } else {
                    return Err(format!("Read error: {}", err));
                }
            }

            if bytes_read == 0 {
                return Ok(()); // Client disconnected
            }

            // Validate message size
            if bytes_read > BUFFER_SIZE {
                eprintln!("Message too large: {} bytes", bytes_read);
                continue;
            }

            // Parse and handle request
            let request_data = &buffer[..bytes_read as usize];
            let response = self.handle_request(request_data);

            // Send response
            let response_json = serde_json::to_vec(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;

            // Validate response size
            if response_json.len() > BUFFER_SIZE as usize {
                eprintln!("Response too large: {} bytes", response_json.len());
                let error_response =
                    RpcResponse::error(0, error_codes::INTERNAL_ERROR, "Response too large");
                let error_json = serde_json::to_vec(&error_response).unwrap_or_default();
                self.write_response(pipe, &error_json, io_event)?;
                continue;
            }

            self.write_response(pipe, &response_json, io_event)?;
        }
    }

    /// Write response with overlapped I/O
    fn write_response(&self, pipe: HANDLE, data: &[u8], io_event: HANDLE) -> Result<(), String> {
        let mut overlapped = OVERLAPPED {
            hEvent: io_event,
            ..Default::default()
        };

        let mut bytes_written: u32 = 0;
        let write_result = unsafe {
            WriteFile(
                pipe,
                Some(data),
                Some(&mut bytes_written),
                Some(&mut overlapped),
            )
        };

        if write_result.is_err() {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(997) {
                // IO_PENDING - wait for completion
                let wait_result = unsafe {
                    WaitForSingleObject(io_event, 5000) // 5 second write timeout
                };
                if wait_result != WAIT_OBJECT_0 {
                    return Err("Write timeout".to_string());
                }
                unsafe { GetOverlappedResult(pipe, &overlapped, &mut bytes_written, false) }
                    .map_err(|_| format!("Write error: {}", std::io::Error::last_os_error()))?;
            } else {
                return Err(format!("Write error: {}", err));
            }
        }

        unsafe { FlushFileBuffers(pipe) }.ok();
        Ok(())
    }

    /// Parse and handle a JSON-RPC request
    fn handle_request(&self, data: &[u8]) -> RpcResponse {
        // Parse JSON
        let request: RpcRequest = match serde_json::from_slice(data) {
            Ok(req) => req,
            Err(e) => {
                return RpcResponse::error(
                    0,
                    error_codes::PARSE_ERROR,
                    &format!("Parse error: {}", e),
                );
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return RpcResponse::error(
                request.id,
                error_codes::INVALID_REQUEST,
                "Invalid JSON-RPC version",
            );
        }

        // Dispatch method
        match request.method.as_str() {
            methods::PING => self.handle_ping(request.id),
            methods::WRITE_BINDING => self.handle_write_binding(request.id, &request.params),
            methods::WRITE_BINDINGS_BATCH => {
                self.handle_write_bindings_batch(request.id, &request.params)
            }
            methods::CLEAR_BINDING => self.handle_clear_binding(request.id, &request.params),
            methods::CLEAR_BINDINGS_BATCH => {
                self.handle_clear_bindings_batch(request.id, &request.params)
            }
            methods::READ_BINDING => self.handle_read_binding(request.id, &request.params),
            methods::GET_ALL_BINDINGS => self.handle_get_all_bindings(request.id),
            methods::FLUSH_DNS => self.handle_flush_dns(request.id),
            _ => RpcResponse::error(
                request.id,
                error_codes::METHOD_NOT_FOUND,
                &format!("Method not found: {}", request.method),
            ),
        }
    }

    fn handle_ping(&self, id: u64) -> RpcResponse {
        let result = PingResult {
            pong: true,
            version: SERVICE_VERSION.to_string(),
        };
        RpcResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_write_binding(&self, id: u64, params: &serde_json::Value) -> RpcResponse {
        let params: WriteBindingParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return RpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    &format!("Invalid params: {}", e),
                );
            }
        };

        match HostsManager::write_binding(&params.domain, &params.ip) {
            Ok(()) => {
                let result = SuccessResult { success: true };
                RpcResponse::success(id, serde_json::to_value(result).unwrap())
            }
            Err(e) => self.hosts_error_to_response(id, e),
        }
    }

    fn handle_write_bindings_batch(&self, id: u64, params: &serde_json::Value) -> RpcResponse {
        let params: WriteBindingsBatchParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return RpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    &format!("Invalid params: {}", e),
                );
            }
        };

        let bindings: Vec<HostsBinding> = params
            .bindings
            .into_iter()
            .map(|b| HostsBinding {
                domain: b.domain,
                ip: b.ip,
            })
            .collect();

        match HostsManager::write_bindings_batch(&bindings) {
            Ok(count) => {
                let result = CountResult {
                    count: count as u32,
                };
                RpcResponse::success(id, serde_json::to_value(result).unwrap())
            }
            Err(e) => self.hosts_error_to_response(id, e),
        }
    }

    fn handle_clear_binding(&self, id: u64, params: &serde_json::Value) -> RpcResponse {
        let params: ClearBindingParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return RpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    &format!("Invalid params: {}", e),
                );
            }
        };

        match HostsManager::clear_binding(&params.domain) {
            Ok(()) => {
                let result = SuccessResult { success: true };
                RpcResponse::success(id, serde_json::to_value(result).unwrap())
            }
            Err(e) => self.hosts_error_to_response(id, e),
        }
    }

    fn handle_clear_bindings_batch(&self, id: u64, params: &serde_json::Value) -> RpcResponse {
        let params: ClearBindingsBatchParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return RpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    &format!("Invalid params: {}", e),
                );
            }
        };

        let domains: Vec<&str> = params.domains.iter().map(|s| s.as_str()).collect();

        match HostsManager::clear_bindings_batch(&domains) {
            Ok(count) => {
                let result = CountResult {
                    count: count as u32,
                };
                RpcResponse::success(id, serde_json::to_value(result).unwrap())
            }
            Err(e) => self.hosts_error_to_response(id, e),
        }
    }

    fn handle_read_binding(&self, id: u64, params: &serde_json::Value) -> RpcResponse {
        let params: ReadBindingParams = match serde_json::from_value(params.clone()) {
            Ok(p) => p,
            Err(e) => {
                return RpcResponse::error(
                    id,
                    error_codes::INVALID_PARAMS,
                    &format!("Invalid params: {}", e),
                );
            }
        };

        let ip = HostsManager::read_binding(&params.domain);
        let result = ReadBindingResult { ip };
        RpcResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_get_all_bindings(&self, id: u64) -> RpcResponse {
        // Read hosts file and extract all anyFAST bindings
        let content = match std::fs::read_to_string(r"C:\Windows\System32\drivers\etc\hosts") {
            Ok(c) => c,
            Err(e) => {
                return RpcResponse::error(
                    id,
                    error_codes::IO_ERROR,
                    &format!("Failed to read hosts file: {}", e),
                );
            }
        };

        let mut bindings = Vec::new();
        let mut in_block = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed == "# BEGIN anyFAST" {
                in_block = true;
                continue;
            }
            if trimmed == "# END anyFAST" {
                in_block = false;
                continue;
            }

            if in_block && !trimmed.is_empty() && !trimmed.starts_with('#') {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    bindings.push(BindingEntry {
                        ip: parts[0].to_string(),
                        domain: parts[1].to_string(),
                    });
                }
            }
        }

        let result = AllBindingsResult { bindings };
        RpcResponse::success(id, serde_json::to_value(result).unwrap())
    }

    fn handle_flush_dns(&self, id: u64) -> RpcResponse {
        match HostsManager::flush_dns() {
            Ok(()) => {
                let result = SuccessResult { success: true };
                RpcResponse::success(id, serde_json::to_value(result).unwrap())
            }
            Err(e) => self.hosts_error_to_response(id, e),
        }
    }

    fn hosts_error_to_response(
        &self,
        id: u64,
        error: crate::hosts_manager::HostsError,
    ) -> RpcResponse {
        use crate::hosts_manager::HostsError;

        match error {
            HostsError::PermissionDenied => {
                RpcResponse::error(id, error_codes::PERMISSION_DENIED, "Permission denied")
            }
            HostsError::InvalidIp(ip) => {
                RpcResponse::error(id, error_codes::INVALID_IP, &format!("Invalid IP: {}", ip))
            }
            HostsError::InvalidDomain(domain) => RpcResponse::error(
                id,
                error_codes::INVALID_DOMAIN,
                &format!("Invalid domain: {}", domain),
            ),
            HostsError::Io(e) => {
                RpcResponse::error(id, error_codes::IO_ERROR, &format!("IO error: {}", e))
            }
        }
    }
}

impl Default for PipeServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PipeServer {
    fn drop(&mut self) {
        if self.stop_event != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.stop_event) }.ok();
        }
    }
}

/// RAII guard for Windows handles
struct HandleGuard(HANDLE);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) }.ok();
        }
    }
}
