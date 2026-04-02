//! Proxy authentication — bearer token validation and provider credential resolution.
//!
//! Clients authenticate to the proxy with a bearer token. The proxy maps
//! this to provider credentials in the registry. This keeps provider API keys
//! server-side — clients never see them.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

use super::ProxyState;

/// Middleware: validate bearer token for proxy requests.
///
/// Checks `Authorization: Bearer <token>` against the gateway config.
/// If `auth_required` is false, all requests pass through.
pub async fn require_proxy_auth(
    request: Request,
    next: Next,
) -> Response {
    let state = request
        .extensions()
        .get::<Arc<ProxyState>>()
        .cloned();

    let state = match state {
        Some(s) => s,
        None => return next.run(request).await,
    };

    if !state.config.auth_required {
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    let token = auth_header.strip_prefix("Bearer ").unwrap_or_default().trim();

    if token.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({
                "error": {
                    "message": "Missing bearer token. Set Authorization: Bearer <token>",
                    "type": "invalid_request_error",
                    "code": "missing_token"
                }
            })),
        )
            .into_response();
    }

    // Validate against configured proxy token
    if let Some(ref expected) = state.config.proxy_token {
        if token != expected {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": {
                        "message": "Invalid bearer token",
                        "type": "invalid_request_error",
                        "code": "invalid_token"
                    }
                })),
            )
                .into_response();
        }
    }

    next.run(request).await
}
