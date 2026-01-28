//! JSON-RPC 2.0 protocol definitions for hosts service IPC
//!
//! This module defines the request/response types for communication
//! between the GUI client and the privileged hosts service.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

impl RpcRequest {
    pub fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(RpcError {
                code,
                message: message.to_string(),
            }),
        }
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // Custom error codes (application-specific)
    pub const PERMISSION_DENIED: i32 = -1;
    pub const INVALID_IP: i32 = -2;
    pub const INVALID_DOMAIN: i32 = -3;
    pub const IO_ERROR: i32 = -4;
}

/// RPC method names
pub mod methods {
    pub const PING: &str = "ping";
    pub const WRITE_BINDING: &str = "write_binding";
    pub const WRITE_BINDINGS_BATCH: &str = "write_bindings_batch";
    pub const CLEAR_BINDING: &str = "clear_binding";
    pub const CLEAR_BINDINGS_BATCH: &str = "clear_bindings_batch";
    pub const READ_BINDING: &str = "read_binding";
    pub const GET_ALL_BINDINGS: &str = "get_all_bindings";
    pub const FLUSH_DNS: &str = "flush_dns";
}

// ============ Request parameter types ============

/// Parameters for write_binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteBindingParams {
    pub domain: String,
    pub ip: String,
}

/// Parameters for write_bindings_batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteBindingsBatchParams {
    pub bindings: Vec<BindingEntry>,
}

/// A single binding entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingEntry {
    pub domain: String,
    pub ip: String,
}

/// Parameters for clear_binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearBindingParams {
    pub domain: String,
}

/// Parameters for clear_bindings_batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearBindingsBatchParams {
    pub domains: Vec<String>,
}

/// Parameters for read_binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadBindingParams {
    pub domain: String,
}

// ============ Response result types ============

/// Result for write_binding, clear_binding, flush_dns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResult {
    pub success: bool,
}

/// Result for write_bindings_batch, clear_bindings_batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountResult {
    pub count: u32,
}

/// Result for read_binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadBindingResult {
    pub ip: Option<String>,
}

/// Result for get_all_bindings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllBindingsResult {
    pub bindings: Vec<BindingEntry>,
}

/// Result for ping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub pong: bool,
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = RpcRequest::new(
            1,
            methods::WRITE_BINDING,
            serde_json::json!({
                "domain": "example.com",
                "ip": "1.2.3.4"
            }),
        );

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("write_binding"));
        assert!(json.contains("example.com"));

        let parsed: RpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, 1);
        assert_eq!(parsed.method, methods::WRITE_BINDING);
    }

    #[test]
    fn test_response_success() {
        let resp = RpcResponse::success(
            1,
            serde_json::to_value(SuccessResult { success: true }).unwrap(),
        );

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("success"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn test_response_error() {
        let resp = RpcResponse::error(1, error_codes::PERMISSION_DENIED, "Access denied");

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Access denied"));
        assert!(json.contains("-1"));
    }
}
