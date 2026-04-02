//! Shared types for discovery and proxy.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Discovery types
// ---------------------------------------------------------------------------

/// A discovered Genesis/OpenCode server instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Full URL (e.g., "http://localhost:39175" or "http://imac.tail1234.ts.net:39175")
    pub url: String,
    /// Project directory the server is serving
    pub project_dir: Option<String>,
    /// Project name (last path component of project_dir)
    pub project_name: Option<String>,
    /// Server version (from /health endpoint)
    pub version: Option<String>,
    /// How the server was discovered
    pub source: DiscoverySource,
    /// Latency to health endpoint in milliseconds
    pub latency_ms: Option<u64>,
    /// Whether the server responded to probe
    pub alive: bool,
}

/// How a server was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoverySource {
    /// server-hint file on local filesystem
    ServerHint,
    /// Port scan on localhost
    PortScan,
    /// mDNS/Bonjour on LAN
    Mdns,
    /// Tailscale peer discovery
    Tailscale,
    /// Manually configured URL
    Manual,
}

/// Discovery scope configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Scan localhost (always allowed)
    #[serde(default = "default_true")]
    pub local: bool,
    /// Scan LAN via mDNS (opt-in, requires permission)
    #[serde(default)]
    pub lan: bool,
    /// Scan Tailscale peers (opt-in, requires permission)
    #[serde(default)]
    pub tailscale: bool,
    /// Port range for scanning
    #[serde(default = "default_port_range")]
    pub port_range: (u16, u16),
    /// Probe timeout in milliseconds
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

fn default_true() -> bool { true }
fn default_port_range() -> (u16, u16) { (39175, 39687) }
fn default_probe_timeout() -> u64 { 2000 }

// ---------------------------------------------------------------------------
// Proxy types
// ---------------------------------------------------------------------------

/// Gateway/proxy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Whether the /v1/* proxy routes are enabled
    #[serde(default)]
    pub enabled: bool,
    /// Require bearer token for proxy access
    #[serde(default = "default_true")]
    pub auth_required: bool,
    /// CORS allowed origins for proxy routes
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Optional bearer token for proxy authentication
    /// (if not set, uses the server's basic auth)
    #[serde(default)]
    pub proxy_token: Option<String>,
    /// Advertise as this provider name to clients
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

fn default_provider_name() -> String { "genesis-proxy".to_string() }

/// A model exposed through the proxy, with source tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxiedModel {
    /// Model ID as seen by clients (e.g., "claude-sonnet-4-20250514")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// The upstream provider serving this model
    pub upstream_provider: String,
    /// Whether this is a local model (Ollama, MLX) or cloud
    pub local: bool,
    /// Context window size
    pub context_window: u64,
    /// Capabilities from the model card
    pub capabilities: ProxiedModelCapabilities,
}

/// Capability flags advertised to clients via /v1/models.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxiedModelCapabilities {
    pub reasoning: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub streaming: bool,
    pub json_mode: bool,
    pub function_calling: bool,
}
