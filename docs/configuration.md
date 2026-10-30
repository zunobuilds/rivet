# Configuration

Rivet reads `.rivet.toml` in the selected project, or an explicit contained `--config` file. It does not search parent/home settings. Schema version 1 rejects unknown fields. Config files must be ordinary single-link files within the validated project; symlinks, unsafe paths and concurrent changes fail closed.

The smallest model-free configuration is:

```toml
schema_version = 1
```

`provider` has `profile`, `endpoint`, `model` and optional `context_tokens`; inference requires a positive supported context ceiling. See [providers](providers.md). `mcp` and `skills` are explicit lists with strict version/content-pinning rules; see the [P1d evidence](../planning/research/p1d-adapters.md) and typed structures in `src/mcp.rs` and `src/skills.rs` before configuring them. Discovery is not implicit permission.

State defaults to `.rivet-state/`. Commands supporting `--state-dir` still require a contained safe path. Status/config/list do not initialize absent state; state/session creation and chat are writes. Busy ownership, unsafe permissions, unknown schemas and recovery remnants have explicit errors. Active-run status uses its private control channel.

Current CLI bounds default to 100 steps, 3600 seconds and 65536 tokens; remote request cost defaults to zero and cannot silently authorize paid use. Run `chat --help` for exact overrides. Steps include inference and tool dispatch. A resume retains stored limits rather than refreshing its budget or deadline.

Global provider inheritance, arbitrary workspaces, tool toggles, resource profiles and mods are proposals; no configuration keys for them are supported yet.
