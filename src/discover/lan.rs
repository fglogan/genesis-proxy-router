//! LAN discovery via mDNS/Bonjour (`_genesis._tcp`).
//!
//! Requires `discover.lan = true` in config (opt-in with explicit permission).

use crate::{DiscoveryConfig, ServerInfo};

/// Service type for mDNS advertisement and discovery.
pub const MDNS_SERVICE_TYPE: &str = "_genesis._tcp.local.";

/// Scan LAN for Genesis servers via mDNS.
///
/// Not yet implemented — returns empty for now.
#[allow(clippy::unused_async)]
pub async fn scan_lan(_config: &DiscoveryConfig) -> Vec<ServerInfo> {
    tracing::debug!("LAN discovery not yet implemented (mDNS {MDNS_SERVICE_TYPE})");
    Vec::new()
}

/// Advertise this Genesis server on the LAN via mDNS.
///
/// Called by the server at startup when `gateway.enabled = true`.
/// Advertises `_genesis._tcp` with TXT records for version, project, and port.
#[allow(clippy::unused_async)]
pub async fn advertise(_port: u16, _project: &str, _version: &str) {
    tracing::debug!("mDNS advertisement not yet implemented");
}
