//! Shared types for discovery and proxy.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ServerInfo {
    /// Full URL (e.g., `http://localhost:39175`).
    #[doc = "Full URL to the server endpoint."]
    pub url: String,
    /// Project directory the server is serving.
    #[doc = "Project directory the server is serving."]
    pub project_dir: Option<String>,
    /// Project name (last path component of `project_dir`).
    #[doc = "Project name derived from the directory path."]
    pub project_name: Option<String>,
    /// Server version (from `/health` endpoint).
    #[doc = "Server version reported by the health endpoint."]
    pub version: Option<String>,
    /// How the server was discovered.
    #[doc = "Discovery source that found this server."]
    pub source: DiscoverySource,
    /// Latency to health endpoint in milliseconds.
    #[doc = "Round-trip latency to the health endpoint in milliseconds."]
    pub latency_ms: Option<u64>,
    /// Whether the server responded to probe.
    #[doc = "Whether the server responded to the health probe."]
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DiscoverySource {
    /// Server-hint file on local filesystem.
    ServerHint,
    /// Port scan on localhost.
    PortScan,
    /// mDNS/Bonjour on LAN.
    Mdns,
    /// Tailscale peer discovery.
    Tailscale,
    /// Manually configured URL.
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DiscoveryConfig {
    /// Scan localhost (always allowed).
    #[serde(default = "default_true")]
    pub local: bool,
    /// Scan LAN via mDNS (opt-in, requires permission).
    #[serde(default)]
    pub lan: bool,
    /// Scan Tailscale peers (opt-in, requires permission).
    #[serde(default)]
    pub tailscale: bool,
    /// Port range for scanning.
    #[serde(default = "default_port_range")]
    pub port_range: (u16, u16),
    /// Probe timeout in milliseconds.
    #[serde(default = "default_probe_timeout")]
    pub probe_timeout_ms: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            local: true,
            lan: false,
            tailscale: false,
            port_range: default_port_range(),
            probe_timeout_ms: default_probe_timeout(),
        }
    }
}

#[inline]
#[must_use]
pub(crate) fn default_true() -> bool {
    true
}

#[inline]
#[must_use]
const fn default_port_range() -> (u16, u16) {
    (39_175, 39_687)
}

#[inline]
#[must_use]
const fn default_probe_timeout() -> u64 {
    2000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GatewayConfig {
    /// Whether the `/v1/*` proxy routes are enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Require bearer token for proxy access.
    #[serde(default = "default_true")]
    pub auth_required: bool,
    /// CORS allowed origins for proxy routes.
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Optional bearer token for proxy authentication.
    #[serde(default)]
    pub proxy_token: Option<String>,
    /// Advertise as this provider name to clients.
    #[serde(default = "default_provider_name")]
    pub provider_name: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_required: true,
            allowed_origins: Vec::new(),
            proxy_token: None,
            provider_name: default_provider_name(),
        }
    }
}

#[inline]
#[must_use]
fn default_provider_name() -> String {
    String::from("genesis-proxy")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProxiedModel {
    /// Model ID as seen by clients (e.g., `claude-sonnet-4-20250514`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The upstream provider serving this model.
    pub upstream_provider: String,
    /// Whether this is a local model (Ollama, MLX) or cloud.
    pub local: bool,
    /// Context window size.
    pub context_window: u64,
    /// Capabilities from the model card.
    pub capabilities: ProxiedModelCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProxiedModelCapabilities {
    /// Supports multi-step reasoning.
    pub reasoning: bool,
    /// Supports tool/function calling.
    pub tool_calling: bool,
    /// Supports image/vision input.
    pub vision: bool,
    /// Supports streaming responses.
    pub streaming: bool,
    /// Supports JSON mode output.
    pub json_mode: bool,
    /// Supports function calling (legacy).
    pub function_calling: bool,
}
