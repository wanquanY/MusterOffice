//! Thin computation MCP by default. The old persistent host is opt-in only.
#[cfg(feature = "legacy-host")]
pub mod bridge;
#[cfg(feature = "legacy-host")]
pub mod catalog;
#[cfg(feature = "legacy-host")]
pub mod channel;
pub mod compute;
#[cfg(feature = "legacy-host")]
pub mod config;
#[cfg(feature = "legacy-host")]
pub mod resources;
#[cfg(feature = "legacy-host")]
pub mod server;
pub mod transport;

#[cfg(feature = "http")]
pub mod http;
