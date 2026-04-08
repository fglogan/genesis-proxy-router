//! OpenAI-compatible `/v1/` route handlers.
//!
//! These handlers accept standard OpenAI API requests, route them through
//! the Genesis provider registry, and return OpenAI-format responses.

use axum::{
    Json,
    Router,
    extract::State,
    http::StatusCode,
    response::{
        IntoResponse,
        sse::{KeepAlive, Sse},
    },
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

use super::ProxyState;
use crate::proxy::adapter;

/// Mount all OpenAI-compatible routes.
pub fn routes() -> Router<Arc<ProxyState>> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", get(list_models))
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ChatCompletionRequest {
    /// Model ID to use for completion.
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<Value>,
    /// Enable streaming response.
    #[serde(default)]
    pub stream: bool,
    /// Available tools for the model.
    #[serde(default)]
    pub tools: Vec<Value>,
    /// Tool choice strategy.
    #[serde(default)]
    pub tool_choice: Option<String>,
    /// Sampling temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Nucleus sampling threshold.
    #[serde(default)]
    pub top_p: Option<f64>,
    /// Maximum tokens to generate.
    #[serde(default)]
    pub max_tokens: Option<u64>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct ChatCompletionChoice {
    index: u32,
    message: Value,
    finish_reason: Option<String>,
}

/// Handle `POST /v1/chat/completions`.
///
/// Resolves the model to a provider, streams the response, and converts
/// Genesis `StreamChunk`s to OpenAI SSE format.
async fn chat_completions(
    state: State<Arc<ProxyState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let Some((provider_id, model_id)) = state.provider_registry.resolve_model(&req.model) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": {
                    "message": format!("Model '{}' not found. Use GET /v1/models for available models.", req.model),
                    "type": "invalid_request_error",
                    "code": "model_not_found"
                }
            })),
        )
            .into_response();
    };

    let Some(provider) = state.provider_registry.get_provider(&provider_id) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": {
                    "message": format!("Provider '{}' is not connected", provider_id),
                    "type": "server_error",
                    "code": "provider_unavailable"
                }
            })),
        )
            .into_response();
    };

    let tool_choice = req.tool_choice.as_deref();

    let Ok(chunk_stream) = provider
        .stream(&model_id, req.messages, req.tools, tool_choice)
        .await
    else {
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": {
                    "message": "Upstream provider error",
                    "type": "server_error",
                    "code": "upstream_error"
                }
            })),
        )
            .into_response();
    };

    let model_name = req.model.clone();
    if req.stream {
        let sse_stream = adapter::stream_to_openai_sse(chunk_stream, &model_name);
        Sse::new(sse_stream)
            .keep_alive(KeepAlive::new())
            .into_response()
    } else {
        let response = adapter::collect_to_openai_response(chunk_stream, &model_name).await;
        (StatusCode::OK, Json(response)).into_response()
    }
}

/// Handle `GET /v1/models`.
///
/// Returns all connected models from all providers, with Genesis-specific
/// capability metadata in the `genesis` extension field.
async fn list_models(
    state: State<Arc<ProxyState>>,
) -> impl IntoResponse {
    let models = state.provider_registry.list_models();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let data: Vec<Value> = models
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "object": "model",
                "created": now,
                "owned_by": state.config.provider_name,
                "genesis": {
                    "name": m.name,
                    "upstream_provider": m.upstream_provider,
                    "local": m.local,
                    "context_window": m.context_window,
                    "capabilities": m.capabilities,
                }
            })
        })
        .collect();

    Json(json!({
        "object": "list",
        "data": data,
    }))
}
