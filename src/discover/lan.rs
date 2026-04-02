//! LAN discovery via mDNS/Bonjour (`_genesis._tcp`).
//!
//! Requires `discover.lan = true` in config (opt-in with explicit permission).

use crate::{DiscoveryConfig, DiscoverySource, ServerInfo};

/// Service type for mDNS advertisement and discovery.
pub const MDNS_SERVICE_TYPE: &str = "_genesis._tcp.local.";

/// Scan LAN for Genesis servers via mDNS.
///
/// TODO: Implement using `mdns-sd` or `zeroconf` crate.
/// For now returns empty — the interface is defined for consumers to depend on.
pub async fn scan_lan(_config: &DiscoveryConfig) -> Vec<ServerInfo> {
    tracing::debug!("LAN discovery not yet implemented (mDNS {MDNS_SERVICE_TYPE})");
    Vec::new()
}

/// Advertise this Genesis server on the LAN via mDNS.
///
/// Called by the server at startup when `gateway.enabled = true`.
/// Advertises `_genesis._tcp` with TXT records:
/// - `version=2.2.0`
/// - `project=<project_name>`
/// - `port=<port>`
pub async fn advertise(_port: u16, _project: &str, _version: &str) {
    tracing::debug!("mDNS advertisement not yet implemented");
}
