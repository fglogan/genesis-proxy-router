//! Genesis Proxy Router — discovery + transparent OpenAI-compatible proxy.
//!
//! This crate provides two independent capabilities:
//!
//! ## Discovery (`discover` feature)
//! Find Genesis/OpenCode servers across local, LAN, and Tailscale networks.
//! Returns [`ServerInfo`] with URL, project name, version, and capabilities.
//!
//! ## Proxy (`proxy` feature)
//! Expose connected providers as standard OpenAI-compatible endpoints:
//! - `POST /v1/chat/completions` — streaming and non-streaming
//! - `GET /v1/models` — model catalog with capabilities from provider cards
//!
//! Clients connect using any OpenAI SDK and get transparent access to all
//! Genesis-managed providers (local + cloud).

#[cfg(feature = "discover")]
pub mod discover;

#[cfg(feature = "proxy")]
pub mod proxy;

pub mod stream;
pub mod types;
pub mod util;

pub use stream::{ChunkStream, LlmProvider, StreamChunk, TokenUsage};
pub use types::{
    DiscoveryConfig,
    DiscoverySource,
    GatewayConfig,
    ProxiedModel,
    ProxiedModelCapabilities,
    ServerInfo,
};
pub use util::generate_id;
