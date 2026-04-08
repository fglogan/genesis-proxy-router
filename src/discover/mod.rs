//! Server discovery across local, LAN, and Tailscale networks.

pub mod lan;
pub mod local;
pub mod tailscale;

use crate::{DiscoveryConfig, ServerInfo};

/// Scan for Genesis/OpenCode servers according to the discovery config.
///
/// Returns servers from all enabled scopes, sorted by latency.
pub async fn scan(config: &DiscoveryConfig) -> Vec<ServerInfo> {
    let mut servers = Vec::new();

    if config.local {
        servers.extend(local::scan_local(config).await);
    }
    if config.lan {
        servers.extend(lan::scan_lan(config).await);
    }
    if config.tailscale {
        servers.extend(tailscale::scan_tailscale(config).await);
    }

    servers.sort_by(|a, b| a.latency_ms.cmp(&b.latency_ms));
    servers.dedup_by(|a, b| a.url == b.url);
    servers
}
