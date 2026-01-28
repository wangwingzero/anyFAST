//! Windows Service module for anyFAST
//!
//! Provides a privileged service that manages hosts file operations,
//! allowing the GUI to run without administrator privileges.

pub mod rpc;

#[cfg(windows)]
pub mod pipe_server;
