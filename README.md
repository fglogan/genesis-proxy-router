# genesis-proxy-router

Transparent OpenAI-compatible proxy + multi-network server discovery for Genesis.

Any OpenAI SDK client connects to Genesis and gets access to all managed providers (Anthropic, OpenAI, Google, Ollama, MLX, 40+ total) through a single `/v1/` endpoint. Clients never need provider-specific credentials — Genesis handles routing, auth, and streaming.

## Quick Start

### 1. Enable the gateway

Add to `~/.config/opencode/opencode.jsonc`:

```jsonc
{
  "gateway": {
    "enabled": true,
    "auth_required": false  // set true + proxy_token for production
  }
}
```

### 2. Start the Genesis server

```bash
genesis server
# or
genesis web
```

### 3. Use it with any OpenAI client

**curl:**
```bash
# List all available models
curl http://localhost:39175/v1/models | jq .

# Chat completion (streaming)
curl http://localhost:39175/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "messages": [{"role": "user", "content": "hello"}],
    "stream": true
  }'

# Chat completion (non-streaming)
curl http://localhost:39175/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "hello"}]
  }'
```

**Python (openai SDK):**
```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:39175/v1",
    api_key="not-needed",  # when auth_required is false
)

# List models — shows all Genesis-managed providers
for model in client.models.list():
    print(f"{model.id} ({model.genesis['upstream_provider']}, "
          f"{'local' if model.genesis['local'] else 'cloud'})")

# Chat — routes transparently to the right provider
response = client.chat.completions.create(
    model="claude-sonnet-4-20250514",
    messages=[{"role": "user", "content": "What is Genesis?"}],
)
print(response.choices[0].message.content)
```

**Node.js (openai SDK):**
```typescript
import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "http://localhost:39175/v1",
  apiKey: "not-needed",
});

const stream = await client.chat.completions.create({
  model: "claude-sonnet-4-20250514",
  messages: [{ role: "user", content: "hello" }],
  stream: true,
});

for await (const chunk of stream) {
  process.stdout.write(chunk.choices[0]?.delta?.content || "");
}
```

## API Reference

### `GET /v1/models`

Returns all models from all connected providers.

```json
{
  "object": "list",
  "data": [
    {
      "id": "claude-sonnet-4-20250514",
      "object": "model",
      "owned_by": "genesis-proxy",
      "genesis": {
        "name": "Claude Sonnet 4",
        "upstream_provider": "anthropic",
        "local": false,
        "context_window": 200000,
        "capabilities": {
          "reasoning": true,
          "tool_calling": true,
          "vision": true,
          "streaming": true,
          "json_mode": false,
          "function_calling": true
        }
      }
    },
    {
      "id": "llama3.2:latest",
      "object": "model",
      "owned_by": "genesis-proxy",
      "genesis": {
        "upstream_provider": "ollama",
        "local": true,
        "context_window": 131072,
        "capabilities": { "reasoning": false, "tool_calling": true, ... }
      }
    }
  ]
}
```

### `POST /v1/chat/completions`

Standard OpenAI chat completions API. Supports streaming and non-streaming.

**Request:**
```json
{
  "model": "claude-sonnet-4-20250514",
  "messages": [
    {"role": "system", "content": "You are helpful."},
    {"role": "user", "content": "Hello"}
  ],
  "stream": true,
  "tools": [],
  "temperature": 0.7
}
```

**Response (streaming):** OpenAI-format SSE events:
```
data: {"id":"chatcmpl-gen_...","object":"chat.completion.chunk","model":"claude-sonnet-4-20250514","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: [DONE]
```

**Response (non-streaming):**
```json
{
  "id": "chatcmpl-gen_...",
  "object": "chat.completion",
  "model": "claude-sonnet-4-20250514",
  "choices": [{
    "index": 0,
    "message": {"role": "assistant", "content": "Hello! How can I help?"},
    "finish_reason": "stop"
  }],
  "usage": {
    "prompt_tokens": 12,
    "completion_tokens": 8,
    "total_tokens": 20
  }
}
```

## CLI

The Genesis CLI exposes proxy functionality:

```bash
# Start server with gateway enabled
genesis server                          # uses config file setting
GENESIS_GATEWAY_ENABLED=true genesis server  # env var override (planned)

# Discovery (planned)
genesis discover                        # scan local servers
genesis discover --lan                  # scan LAN via mDNS
genesis discover --tailscale            # scan Tailscale peers
```

## Discovery

Find Genesis servers across networks. Three scopes, each opt-in:

| Scope | Method | Config |
|-------|--------|--------|
| **Local** | Server-hint files + port scan (39175-39687) | Always enabled |
| **LAN** | mDNS/Bonjour `_genesis._tcp` | `discover.lan = true` |
| **Tailscale** | `tailscale status --json` + peer probe | `discover.tailscale = true` |

```rust
use genesis_proxy_router::discover;
use genesis_proxy_router::DiscoveryConfig;

let config = DiscoveryConfig::default(); // local only
let servers = discover::scan(&config).await;
for server in &servers {
    println!("{} — {} ({:?}, {}ms)",
        server.url,
        server.project_name.as_deref().unwrap_or("unknown"),
        server.source,
        server.latency_ms.unwrap_or(0),
    );
}
```

## Configuration

### Gateway Config

```jsonc
{
  "gateway": {
    "enabled": false,        // /v1/* routes disabled by default
    "auth_required": true,   // require Bearer token
    "proxy_token": "sk-...", // token for proxy auth (optional)
    "allowed_origins": [],   // CORS allowlist
    "provider_name": "genesis-proxy"  // shown in owned_by field
  }
}
```

### Discovery Config

```jsonc
{
  "discover": {
    "local": true,           // always on
    "lan": false,            // mDNS opt-in
    "tailscale": false,      // Tailscale opt-in
    "port_range": [39175, 39687],
    "probe_timeout_ms": 2000
  }
}
```

## Architecture

```
crates/genesis-proxy-router/
  src/
    lib.rs                 — public API, feature gates
    types.rs               — ServerInfo, GatewayConfig, ProxiedModel, capabilities
    discover/
      mod.rs               — scan() orchestrator
      local.rs             — server-hint files + port range probe
      lan.rs               — mDNS _genesis._tcp (stub)
      tailscale.rs         — tailscale status --json → peer probe
    proxy/
      mod.rs               — ProxyState, ProviderLookup trait, router()
      openai.rs            — POST /v1/chat/completions, GET /v1/models
      adapter.rs           — StreamChunk ↔ OpenAI SSE/JSON format
      auth.rs              — Bearer token validation middleware
```

**Integration with genesis-server:**

```rust
// AppState implements ProviderLookup
impl ProviderLookup for AppState {
    fn get_provider(&self, id: &str) -> Option<Arc<dyn LlmProvider>>;
    fn list_models(&self) -> Vec<ProxiedModel>;
    fn resolve_model(&self, model: &str) -> Option<(String, String)>;
}

// Mounted conditionally in build_router()
if gateway_config.enabled {
    api_router = api_router.nest("/v1", proxy_router);
}
```

## Roadmap

### v2.3.0 (Current)
- [x] Scaffold crate with discovery + proxy modules
- [x] Wire into genesis-server — ProviderLookup on AppState
- [x] `POST /v1/chat/completions` with streaming
- [x] `GET /v1/models` with capability metadata
- [ ] Context analysis plugin hook (IgorWarzocha pattern)
- [ ] `POST /v1/embeddings` support
- [ ] TUI discovery popup panel (Ctrl+P → scan/connect)

### v3.0
- [ ] mDNS/Bonjour LAN discovery implementation
- [ ] Tailscale peer discovery (probing works, needs polish)
- [ ] `genesis discover` CLI subcommand
- [ ] Rate limiting per client/model
- [ ] Token usage tracking and cost attribution
- [ ] Multi-server fleet coordination (proxy → proxy chaining)
- [ ] OpenTelemetry trace propagation through proxy

### v3.1+
- [ ] `POST /v1/responses` (OpenAI Responses API)
- [ ] Model aliasing (map custom names to provider models)
- [ ] Load balancing across multiple instances of same provider
- [ ] Provider health monitoring with automatic failover
- [ ] Web dashboard for proxy metrics
- [ ] Plugin marketplace integration for custom providers

## Security

- **Gateway is OFF by default** — provider credentials are never exposed unless you opt in
- **Bearer token auth** — when `auth_required: true`, clients must present a valid token
- **No credential forwarding** — clients authenticate to the proxy, never to upstream providers
- **CORS configurable** — restrict which origins can access the proxy
- **Local-only by default** — binds to localhost; use a reverse proxy for network exposure
