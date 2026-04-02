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
//! - `POST /v1/embeddings` — if provider supports
//!
//! Clients connect using any OpenAI SDK and get transparent access to all
//! Genesis-managed providers (local + cloud). The proxy virtualizes the
//! provider connection — clients see "Genesis Proxy" as the provider and
//! pull capabilities from model cards.
//!
//! ## Architecture
//!
//! ```text
//! Any OpenAI client ──→ Genesis Proxy Router ──→ LlmProvider trait
//!   (Python SDK,          /v1/chat/completions     ├─ AnthropicProvider
//!    curl, agents)        /v1/models               ├─ OpenAiCompatibleProvider
//!                                                  ├─ OpenAiResponsesProvider
//!                                                  ├─ OllamaProvider (local)
//!                                                  └─ ... (40+ providers)
//! ```
//!
//! ## Security
//!
//! The proxy is **OFF by default**. Enable via config:
//! ```toml
//! [gateway]
//! enabled = true
//! auth_required = true
//! allowed_origins = ["http://localhost:*"]
//! ```

#[cfg(feature = "discover")]
pub mod discover;

#[cfg(feature = "proxy")]
pub mod proxy;

pub mod stream;
pub mod types;
pub mod util;

// Re-export key types
pub use stream::*;
pub use types::*;
