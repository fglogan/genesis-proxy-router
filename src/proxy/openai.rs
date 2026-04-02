//! OpenAI-compatible `/v1/` route handlers.
//!
//! These handlers accept standard OpenAI API requests, route them through
//! the Genesis provider registry, and return OpenAI-format responses.
//! Any OpenAI SDK client works transparently.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, sse::{Event as SseEvent, KeepAlive, Sse}},
    routing::{get, post},
    Router,
};
use futures_util::stream::Stream;
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

// ---------------------------------------------------------------------------
// POST /v1/chat/completions
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub tool_choice: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChatCompletionChoice>,
    usage: Option<CompletionUsage>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionChoice {
    index: u32,
    message: Value,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct CompletionUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

/// Handle `POST /v1/chat/completions`.
///
/// Resolves the model to a provider, streams the response, and converts
/// Genesis StreamChunks to OpenAI SSE format.
async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    // Resolve model → (provider_id, model_id)
    let (provider_id, model_id) = match state.provider_registry.resolve_model(&req.model) {
        Some(resolved) => resolved,
        None => {
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
        }
    };

    // Get the provider
    let provider = match state.provider_registry.get_provider(&provider_id) {
        Some(p) => p,
        None => {
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
        }
    };

    let tool_choice = req.tool_choice.as_deref();

    // Call the provider
    let chunk_stream = match provider
        .stream(&model_id, req.messages, req.tools, tool_choice)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": {
                        "message": format!("Upstream provider error: {e}"),
                        "type": "server_error",
                        "code": "upstream_error"
                    }
                })),
            )
                .into_response();
        }
    };

    let model_name = req.model.clone();
    if req.stream {
        // Streaming response — convert to OpenAI SSE format
        let sse_stream = adapter::stream_to_openai_sse(chunk_stream, &model_name);
        Sse::new(sse_stream)
            .keep_alive(KeepAlive::new())
            .into_response()
    } else {
        // Non-streaming — collect all chunks, return single JSON response
        let response = adapter::collect_to_openai_response(chunk_stream, &model_name).await;
        (StatusCode::OK, Json(response)).into_response()
    }
}

// ---------------------------------------------------------------------------
// GET /v1/models
// ---------------------------------------------------------------------------

/// Handle `GET /v1/models`.
///
/// Returns all connected models from all providers, with Genesis-specific
/// capability metadata in the `genesis` extension field.
async fn list_models(
    State(state): State<Arc<ProxyState>>,
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
