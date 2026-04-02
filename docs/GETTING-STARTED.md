# Genesis Proxy Router — Getting Started

## What is it?

Genesis Proxy Router turns your Genesis server into a transparent OpenAI-compatible API gateway. Any tool that speaks the OpenAI protocol — Python scripts, Node.js apps, curl, other AI agents — can connect to Genesis and use all your managed providers (Anthropic, OpenAI, Google, Ollama, MLX, 40+ total) through a single endpoint.

Clients see a unified model catalog. They don't need to know which provider serves which model. They don't need provider-specific API keys. Genesis handles everything.

## Installation

### As part of Genesis v2 (recommended)

The proxy router ships as part of the Genesis server. No separate installation needed.

```bash
# Build Genesis with proxy support (included by default)
cd genesis-v2
cargo build --release

# The binary includes the proxy router
./target/release/genesis server
```

### As a standalone Rust crate

```toml
# Cargo.toml
[dependencies]
genesis-proxy-router = { path = "../genesis-v2/crates/genesis-proxy-router" }

# Feature flags
# "discover" — server discovery (default: on)
# "proxy"    — OpenAI-compatible proxy routes (default: on)
# "openapi"  — OpenAPI/utoipa schema generation
```

## Configuration

Add to `~/.config/opencode/opencode.jsonc`:

```jsonc
{
  "gateway": {
    "enabled": true,          // mount /v1/* proxy routes
    "auth_required": false,   // true for production — require Bearer token
    "proxy_token": "sk-my-secret-token",  // optional: custom auth token
    "provider_name": "genesis-proxy"      // shown in model ownership
  }
}
```

Start the server:

```bash
genesis server        # or genesis web for browser + server
```

Verify:

```bash
curl http://localhost:39175/v1/models | jq '.data | length'
# → 47 (or however many models your providers offer)
```

## Usage Examples

### Rust

```rust
use reqwest::Client;
use serde_json::json;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new();
    let base = "http://localhost:39175/v1";

    // List models
    let models: serde_json::Value = client
        .get(format!("{base}/models"))
        .send()
        .await?
        .json()
        .await?;

    for model in models["data"].as_array().unwrap_or(&vec![]) {
        let id = model["id"].as_str().unwrap_or("?");
        let provider = model["genesis"]["upstream_provider"].as_str().unwrap_or("?");
        let local = model["genesis"]["local"].as_bool().unwrap_or(false);
        println!("{id} ({provider}, {})", if local { "local" } else { "cloud" });
    }

    // Chat completion (non-streaming)
    let response: serde_json::Value = client
        .post(format!("{base}/chat/completions"))
        .json(&json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [{"role": "user", "content": "Hello from Rust!"}]
        }))
        .send()
        .await?
        .json()
        .await?;

    println!("{}", response["choices"][0]["message"]["content"]);

    // Chat completion (streaming)
    let stream_response = client
        .post(format!("{base}/chat/completions"))
        .json(&json!({
            "model": "claude-sonnet-4-20250514",
            "messages": [{"role": "user", "content": "Count to 5"}],
            "stream": true
        }))
        .send()
        .await?;

    let mut stream = stream_response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" { break; }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        print!("{content}");
                    }
                }
            }
        }
    }
    println!();

    Ok(())
}
```

### Python

```python
from openai import OpenAI

# Connect to Genesis proxy
client = OpenAI(
    base_url="http://localhost:39175/v1",
    api_key="not-needed",  # when auth_required is false
)

# List all models with provider info
for model in client.models.list():
    genesis = model.model_extra.get("genesis", {})
    provider = genesis.get("upstream_provider", "unknown")
    local = genesis.get("local", False)
    caps = genesis.get("capabilities", {})
    print(f"  {model.id:40s} {provider:15s} {'LOCAL' if local else 'CLOUD'}"
          f"  reasoning={caps.get('reasoning', False)}")

# Non-streaming chat
response = client.chat.completions.create(
    model="claude-sonnet-4-20250514",
    messages=[
        {"role": "system", "content": "You are a helpful assistant."},
        {"role": "user", "content": "What providers does Genesis support?"},
    ],
)
print(response.choices[0].message.content)

# Streaming chat
stream = client.chat.completions.create(
    model="claude-sonnet-4-20250514",
    messages=[{"role": "user", "content": "Count to 10"}],
    stream=True,
)
for chunk in stream:
    if chunk.choices[0].delta.content:
        print(chunk.choices[0].delta.content, end="", flush=True)
print()

# Tool calling
response = client.chat.completions.create(
    model="claude-sonnet-4-20250514",
    messages=[{"role": "user", "content": "What's the weather in NYC?"}],
    tools=[{
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get current weather",
            "parameters": {
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"],
            },
        },
    }],
)
print(response.choices[0].message.tool_calls)

# Use a local model (Ollama)
response = client.chat.completions.create(
    model="llama3.2:latest",
    messages=[{"role": "user", "content": "Hello from a local model!"}],
)
print(response.choices[0].message.content)
```

### TypeScript / Node.js

```typescript
import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "http://localhost:39175/v1",
  apiKey: "not-needed",
});

// List models
const models = await client.models.list();
for (const model of models.data) {
  const g = (model as any).genesis;
  console.log(`${model.id} (${g?.upstream_provider}, ${g?.local ? "local" : "cloud"})`);
}

// Streaming chat
const stream = await client.chat.completions.create({
  model: "claude-sonnet-4-20250514",
  messages: [{ role: "user", content: "Hello from TypeScript!" }],
  stream: true,
});

for await (const chunk of stream) {
  const content = chunk.choices[0]?.delta?.content;
  if (content) process.stdout.write(content);
}
console.log();

// Non-streaming with tools
const result = await client.chat.completions.create({
  model: "gpt-4o",
  messages: [{ role: "user", content: "Calculate 42 * 17" }],
  tools: [{
    type: "function",
    function: {
      name: "calculate",
      description: "Evaluate a math expression",
      parameters: {
        type: "object",
        properties: { expression: { type: "string" } },
        required: ["expression"],
      },
    },
  }],
});
console.log(result.choices[0].message);
```

### JavaScript (Browser / Fetch)

```html
<!DOCTYPE html>
<html>
<head><title>Genesis Proxy Demo</title></head>
<body>
<div id="output"></div>
<script>
const BASE = "http://localhost:39175/v1";

// List models
async function listModels() {
  const res = await fetch(`${BASE}/models`);
  const { data } = await res.json();
  data.forEach(m => {
    const g = m.genesis || {};
    console.log(`${m.id} — ${g.upstream_provider} (${g.local ? "local" : "cloud"})`);
  });
  return data;
}

// Streaming chat
async function chat(model, prompt) {
  const res = await fetch(`${BASE}/chat/completions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      model,
      messages: [{ role: "user", content: prompt }],
      stream: true,
    }),
  });

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  const output = document.getElementById("output");
  output.textContent = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    const text = decoder.decode(value);
    for (const line of text.split("\n")) {
      if (!line.startsWith("data: ")) continue;
      const data = line.slice(6);
      if (data === "[DONE]") return;

      try {
        const parsed = JSON.parse(data);
        const content = parsed.choices?.[0]?.delta?.content;
        if (content) output.textContent += content;
      } catch {}
    }
  }
}

// Run
listModels().then(() => chat("claude-sonnet-4-20250514", "What is Genesis?"));
</script>
</body>
</html>
```

### curl

```bash
# Models
curl -s http://localhost:39175/v1/models | jq '.data[] | {id, provider: .genesis.upstream_provider, local: .genesis.local}'

# Non-streaming
curl -s http://localhost:39175/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"hello"}]}' | jq .

# Streaming
curl -N http://localhost:39175/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"claude-sonnet-4-20250514","messages":[{"role":"user","content":"count to 5"}],"stream":true}'

# With auth
curl -s http://localhost:39175/v1/models \
  -H "Authorization: Bearer sk-my-secret-token"

# Use a local model
curl -s http://localhost:39175/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"llama3.2:latest","messages":[{"role":"user","content":"hello from ollama"}]}'
```

## How It Works

```
Client (any language)          Genesis Server              Provider
──────────────────           ──────────────           ──────────
POST /v1/chat/completions → resolve_model("claude-sonnet-4") → anthropic
                           → get_provider("anthropic")
                           → provider.stream(model, messages)
                           ← StreamChunk::TextDelta("Hello")
                           ← adapter → OpenAI SSE format
← data: {"choices":[...]}

GET /v1/models             → list_models()
                           → iterate all providers
                           → map to ProxiedModel with capabilities
← {"data": [...]}
```

The `ProviderLookup` trait is the integration point:
- `get_provider()` — resolves provider_id to an `LlmProvider` implementation
- `list_models()` — collects all models across all providers with metadata
- `resolve_model()` — maps a model name to (provider_id, model_id) with fuzzy matching

## Next Steps

1. **Try it** — enable gateway, restart server, `curl /v1/models`
2. **Connect your tools** — point any OpenAI SDK at `http://localhost:PORT/v1`
3. **Add providers** — configure more providers in Genesis, they appear automatically
4. **Secure it** — set `auth_required: true` and `proxy_token` for production use
5. **Discover servers** — use the discovery module to find Genesis instances on your network
