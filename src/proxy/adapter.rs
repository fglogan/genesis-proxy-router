//! Adapter layer: Genesis StreamChunk ↔ OpenAI SSE/JSON format.
//!
//! This is the core translation layer. Genesis providers emit `StreamChunk`
//! variants; clients expect OpenAI `chat.completion.chunk` SSE events.

use async_stream::stream;
use axum::response::sse::Event as SseEvent;
use futures_util::{Stream, StreamExt};
use crate::stream::StreamChunk;
use serde_json::{Value, json};
use std::convert::Infallible;

/// Convert a Genesis ChunkStream to OpenAI-format SSE events.
///
/// Maps:
/// - `StreamChunk::TextDelta` → `choices[0].delta.content`
/// - `StreamChunk::ToolCall` → `choices[0].delta.tool_calls`
/// - `StreamChunk::ReasoningDelta` → skipped (not in OpenAI spec)
/// - `StreamChunk::Usage` → final chunk with `usage` field
/// - `StreamChunk::Finish` → `[DONE]`
pub fn stream_to_openai_sse(
    chunk_stream: crate::stream::ChunkStream,
    model: &str,
) -> impl Stream<Item = Result<SseEvent, Infallible>> + Send + 'static {
    let model = model.to_string();
    let chunk_id = format!("chatcmpl-{}", crate::util::generate_id());

    stream! {
        let mut chunk_stream = chunk_stream;
        let mut tool_call_index: i32 = 0;

        while let Some(chunk) = chunk_stream.next().await {
            let delta = match &chunk {
                StreamChunk::TextDelta(text) => {
                    json!({ "content": text })
                }
                StreamChunk::ToolCall { id, name, arguments } => {
                    let idx = tool_call_index;
                    tool_call_index += 1;
                    json!({
                        "tool_calls": [{
                            "index": idx,
                            "id": id,
                            "type": "function",
                            "function": { "name": name, "arguments": arguments }
                        }]
                    })
                }
                StreamChunk::ReasoningDelta(_) => {
                    // OpenAI spec doesn't have reasoning in SSE — skip.
                    continue;
                }
                StreamChunk::Usage(usage) => {
                    let data = json!({
                        "id": chunk_id,
                        "object": "chat.completion.chunk",
                        "model": model,
                        "choices": [],
                        "usage": {
                            "prompt_tokens": usage.input,
                            "completion_tokens": usage.output,
                            "total_tokens": usage.total.unwrap_or(usage.input + usage.output),
                        }
                    });
                    if let Ok(json) = serde_json::to_string(&data) {
                        yield Ok::<SseEvent, Infallible>(SseEvent::default().data(json));
                    }
                    continue;
                }
                StreamChunk::Finish { .. } => {
                    yield Ok::<SseEvent, Infallible>(SseEvent::default().data("[DONE]"));
                    break;
                }
                StreamChunk::Error(e) => {
                    let data = json!({
                        "error": {
                            "message": e.to_string(),
                            "type": "server_error",
                            "code": "upstream_error"
                        }
                    });
                    if let Ok(json) = serde_json::to_string(&data) {
                        yield Ok::<SseEvent, Infallible>(SseEvent::default().data(json));
                    }
                    break;
                }
            };

            let data = json!({
                "id": chunk_id,
                "object": "chat.completion.chunk",
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": delta,
                    "finish_reason": serde_json::Value::Null,
                }]
            });

            if let Ok(json) = serde_json::to_string(&data) {
                yield Ok::<SseEvent, Infallible>(SseEvent::default().data(json));
            }
        }
    }
}

/// Collect all chunks into a single OpenAI non-streaming response.
pub async fn collect_to_openai_response(
    chunk_stream: crate::stream::ChunkStream,
    model: &str,
) -> Value {
    let mut content = String::new();
    let mut tool_calls: Vec<Value> = Vec::new();
    let mut usage_input = 0u64;
    let mut usage_output = 0u64;
    let mut finish_reason = "stop".to_string();

    let mut chunk_stream = chunk_stream;
    while let Some(chunk) = chunk_stream.next().await {
        match chunk {
            StreamChunk::TextDelta(text) => content.push_str(&text),
            StreamChunk::ToolCall { id, name, arguments } => {
                tool_calls.push(json!({
                    "id": id,
                    "type": "function",
                    "function": { "name": name, "arguments": arguments }
                }));
                finish_reason = "tool_calls".to_string();
            }
            StreamChunk::Usage(u) => {
                usage_input = u.input;
                usage_output = u.output;
            }
            StreamChunk::Finish { reason } => {
                if !reason.is_empty() {
                    finish_reason = reason;
                }
                break;
            }
            _ => {}
        }
    }

    let mut message = json!({ "role": "assistant", "content": content });
    if !tool_calls.is_empty() {
        message["tool_calls"] = json!(tool_calls);
    }

    json!({
        "id": format!("chatcmpl-{}", crate::util::generate_id()),
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason,
        }],
        "usage": {
            "prompt_tokens": usage_input,
            "completion_tokens": usage_output,
            "total_tokens": usage_input + usage_output,
        }
    })
}
