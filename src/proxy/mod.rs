//! Transparent OpenAI-compatible proxy router.
//!
//! Exposes Genesis-managed providers as standard `/v1/` endpoints.
//! Clients connect using any OpenAI SDK and see a unified model catalog
//! with capabilities from provider cards.

pub mod adapter;
pub mod auth;
pub mod openai;

use std::sync::Arc;

use axum::Router;

use crate::GatewayConfig;

/// Proxy state shared across handler tasks.
#[non_exhaustive]
pub struct ProxyState {
    /// Gateway configuration.
    pub config: GatewayConfig,
    /// Provider registry reference — routes requests to the right `LlmProvider`.
    pub provider_registry: Arc<dyn ProviderLookup>,
}

/// Trait for looking up providers — decouples proxy from server internals.
///
/// Implement this to connect the proxy to your provider backend.
/// In Genesis v2, `AppState` implements this. For standalone use,
/// implement directly against your own provider registry.
pub trait ProviderLookup: Send + Sync {
    /// Get a provider for the given `provider_id`.
    fn get_provider(
        &self,
        provider_id: &str,
    ) -> Option<Arc<dyn crate::stream::LlmProvider>>;

    /// List all available models with their provider and capabilities.
    fn list_models(&self) -> Vec<crate::ProxiedModel>;

    /// Resolve a model ID to `(provider_id, model_id)`.
    ///
    /// Handles aliases, fuzzy matching, and default provider selection.
    fn resolve_model(&self, model: &str) -> Option<(String, String)>;
}

/// Build the proxy router. Mount at `/v1` on the server.
///
/// Returns `None` if the gateway is not enabled in config.
#[must_use]
pub fn router(state: Arc<ProxyState>) -> Option<Router> {
    if !state.config.enabled {
        return None;
    }

    let router = Router::new()
        .merge(openai::routes())
        .with_state(state);

    Some(router)
}
