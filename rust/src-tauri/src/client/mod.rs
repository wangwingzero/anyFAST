//! Client module for communicating with the anyFAST hosts service

#[cfg(windows)]
pub mod pipe_client;

#[cfg(windows)]
pub use pipe_client::PipeClient;
