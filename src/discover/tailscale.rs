//! Tailscale peer discovery — `tailscale status --json` → probe known ports.
//!
//! Requires `discover.tailscale = true` in config (opt-in with explicit permission).

use crate::{DiscoveryConfig, DiscoverySource, ServerInfo};
use crate::discover::local::probe_server;
use std::time::Duration;

/// Scan Tailscale peers for Genesis servers.
///
/// Runs `tailscale status --json`, iterates online peers, probes each
/// on the configured port range for a Genesis health endpoint.
pub async fn scan_tailscale(config: &DiscoveryConfig) -> Vec<ServerInfo> {
    let peers = match get_tailscale_peers().await {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!("Tailscale discovery unavailable: {e}");
            return Vec::new();
        }
    };

    let mut handles = Vec::new();
    let (lo, hi) = config.port_range;
    // Only probe a few well-known ports per peer to avoid flooding
    let probe_ports = [lo, lo + 1, lo + 2];

    for peer in &peers {
        for &port in &probe_ports {
            let url = format!("http://{}:{port}", peer.dns_name);
            let timeout = config.probe_timeout_ms;
            handles.push(tokio::spawn(async move {
                probe_server(
                    &url,
                    DiscoverySource::Tailscale,
                    &DiscoveryConfig {
                        probe_timeout_ms: timeout,
                        ..Default::default()
                    },
                )
                .await
            }));
        }
    }

    let mut servers = Vec::new();
    for handle in handles {
        if let Ok(Some(info)) = handle.await {
            servers.push(info);
        }
    }
    servers
}

/// A Tailscale peer from `tailscale status --json`.
#[derive(Debug, Clone)]
struct TailscalePeer {
    dns_name: String,
    #[allow(dead_code)]
    hostname: String,
    online: bool,
}

/// Parse `tailscale status --json` output.
async fn get_tailscale_peers() -> Result<Vec<TailscalePeer>, String> {
    let output = tokio::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .await
        .map_err(|e| format!("failed to run tailscale: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "tailscale status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("failed to parse tailscale JSON: {e}"))?;

    let peers = json
        .get("Peer")
        .and_then(|p| p.as_object())
        .map(|peers| {
            peers
                .values()
                .filter_map(|peer| {
                    let dns_name = peer.get("DNSName")?.as_str()?.trim_end_matches('.').to_string();
                    let hostname = peer.get("HostName")?.as_str()?.to_string();
                    let online = peer.get("Online").and_then(|v| v.as_bool()).unwrap_or(false);
                    if online {
                        Some(TailscalePeer { dns_name, hostname, online })
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(peers)
}
