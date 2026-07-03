//! Shared logging crate that exposes the Unix socket path, async logger
//! implementation, convenience macros, and protobuf-generated types.

/// Filesystem path for the Unix datagram socket shared by clients and the
/// aggregator.
pub const LOGD_SOCKET_PATH: &str = "/run/pullpirilog/logd.sock";
/// Async logger implementation and background worker.
pub mod logger;
/// Logging convenience macros usable from sync and async call sites.
pub mod macros;

include!("../generated/logd.rs");
