# Providers

No provider, endpoint, model, runtime or weights are selected by default. Configure an existing endpoint; Rivet does not acquire one. Configured means unverified, not inference-ready. No live endpoint was called during this audit.

A schema example for an already authorized local compatible server:

```toml
schema_version = 1

[provider]
profile = "llama.cpp-compatible"
endpoint = "http://127.0.0.1:8080/v1/chat/completions"
model = "replace-with-your-existing-model-id"
context_tokens = 32768
```

Save configuration in project-local `.rivet.toml`, or select a root-contained file with `--config PATH`. The example address is illustrative; no service is assumed running. Set the real context limit supplied by the runtime. Inference rejects unknown ceilings; a syntactically valid configuration alone does not qualify tool use or capacity.

| Profile | Current transport | Important requirement |
| --- | --- | --- |
| `llama.cpp-compatible` | Streamed Chat Completions | Exact `/v1/chat/completions` endpoint, complete supported tool-call stream and usage. |
| `ollama` | Native streamed chat | Exact `/api/chat` endpoint; tool/usage behavior tested synthetically. |
| `anthropic` | Native Messages stream | Exact `/v1/messages` endpoint; explicit ephemeral key input via `--ephemeral-key-stdin`. |

Remote destinations require HTTPS and positive `--spend-ceiling-micros` and `--request-cost-micros`. These are user-declared conservative request bounds, not fetched pricing or a guarantee about provider billing. Project context leaves the machine when sent remotely; approve destination and data explicitly. Loopback does not prove a server itself stays local.

Credentials must not enter TOML, command arguments, issue reports or committed logs. The qualified Anthropic input is ephemeral stdin, not a persistent keyring. Keyring storage, authenticated compatible/Ollama setups and universal provider interoperability are unqualified. Redirects, implicit proxies, hidden retries and fallback are disabled. EOF/cancellation can leave usage unknown and budget held; use explicit recovery rather than blind retry.
