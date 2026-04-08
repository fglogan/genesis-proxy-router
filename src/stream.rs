//! Streaming types for the proxy router.
//!
//! When used standalone (without `genesis-server` feature), these types
//! define the streaming protocol. When used with Genesis, these re-export
//! from `genesis_provider`.

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// A chunk in an LLM response stream.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum StreamChunk {
    /// Text content delta.
    TextDelta(String),
    /// Reasoning/thinking delta.
    ReasoningDelta(String),
    /// Tool call request (complete).
    ToolCall {
        /// Tool call identifier.
        id: String,
        /// Function name to invoke.
        name: String,
        /// JSON-encoded arguments.
        arguments: String,
    },
    /// Token usage update.
    Usage(TokenUsage),
    /// Stream finished.
    Finish {
        /// Reason for completion (e.g., `"stop"`, `"length"`).
        reason: String,
    },
    /// Error from upstream provider.
    Error(String),
}

/// Token usage counts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TokenUsage {
    /// Input/prompt tokens consumed.
    pub input: u64,
    /// Output/completion tokens generated.
    pub output: u64,
    /// Total tokens if reported by provider.
    pub total: Option<u64>,
}

/// A boxed async stream of chunks.
pub type ChunkStream = Pin<Box<dyn Stream<Item = StreamChunk> + Send>>;

/// The core provider trait — implement this to back the proxy with your own providers.
///
/// When used with Genesis v2, `AppState` implements `ProviderLookup` which
/// delegates to `genesis_provider::LlmProvider` implementations. When used
/// standalone, implement this trait directly.
pub trait LlmProvider: Send + Sync {
    /// Provider identifier (e.g., `"anthropic"`, `"openai"`).
    fn id(&self) -> &str;
    /// Human-readable name.
    fn name(&self) -> &str;
    /// Stream a completion from the model.
    fn stream(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        tools: Vec<serde_json::Value>,
        tool_choice: Option<&str>,
    ) -> Pin<Box<dyn std::future::Future<Output = anyhow::Result<ChunkStream>> + Send + '_>>;
}
