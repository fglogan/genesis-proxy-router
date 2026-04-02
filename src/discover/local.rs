//! Local server discovery — server-hint files + port range scanning.

use crate::{DiscoveryConfig, DiscoverySource, ServerInfo};
use std::path::PathBuf;
use std::time::Instant;
use tokio::time::Duration;

/// Scan for local Genesis servers via hint files and port probing.
pub async fn scan_local(config: &DiscoveryConfig) -> Vec<ServerInfo> {
    let mut servers = Vec::new();

    // Phase 1: Check server-hint files (fast, no network)
    for path in hint_file_paths() {
        if let Ok(url) = tokio::fs::read_to_string(&path).await {
            let url = url.trim().to_string();
            if !url.is_empty() {
                if let Some(info) = probe_server(&url, DiscoverySource::ServerHint, config).await {
                    servers.push(info);
                }
            }
        }
    }

    // Phase 2: Port scan localhost range (parallel probes)
    let (lo, hi) = config.port_range;
    let known_ports: std::collections::HashSet<String> = servers.iter().map(|s| s.url.clone()).collect();
    let mut handles = Vec::new();

    for port in lo..=hi {
        let url = format!("http://localhost:{port}");
        if known_ports.contains(&url) {
            continue; // Already found via hint file
        }
        let timeout = config.probe_timeout_ms;
        handles.push(tokio::spawn(async move {
            probe_server(
                &url,
                DiscoverySource::PortScan,
                &DiscoveryConfig {
                    probe_timeout_ms: timeout,
                    ..Default::default()
                },
            )
            .await
        }));
    }

    for handle in handles {
        if let Ok(Some(info)) = handle.await {
            servers.push(info);
        }
    }

    servers
}

/// Probe a server URL for Genesis/OpenCode health endpoint.
pub(crate) async fn probe_server(
    url: &str,
    source: DiscoverySource,
    config: &DiscoveryConfig,
) -> Option<ServerInfo> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(config.probe_timeout_ms))
        .build()
        .ok()?;

    let health_url = format!("{}/health", url.trim_end_matches('/'));
    let start = Instant::now();

    let resp = client.get(&health_url).send().await.ok()?;
    let latency = start.elapsed().as_millis() as u64;

    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = resp.json().await.ok()?;

    // Extract server info from health response
    Some(ServerInfo {
        url: url.trim_end_matches('/').to_string(),
        project_dir: body.get("directory").and_then(|v| v.as_str()).map(String::from),
        project_name: body
            .get("directory")
            .and_then(|v| v.as_str())
            .and_then(|d| d.rsplit('/').next())
            .map(String::from),
        version: body.get("version").and_then(|v| v.as_str()).map(String::from),
        source,
        latency_ms: Some(latency),
        alive: true,
    })
}

/// Known locations for server-hint files.
fn hint_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(state_dir) = local_state_dir() {
        paths.push(state_dir.join("genesis").join("server-hint"));
        paths.push(state_dir.join("genesis-code").join("server-hint"));
        paths.push(state_dir.join("opencode").join("server-hint"));
    }

    paths
}

fn local_state_dir() -> Option<PathBuf> {
    std::env::var("XDG_STATE_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            home_dir().map(|h| h.join(".local/state"))
        })
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}
