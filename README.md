# Rivet

Rivet is an independent AI coding-agent platform for Linux, built in Rust. Start with one agent in Solo; use optional native hierarchies when a task needs coordination. Unrivet is a planned optional reverse-engineering mod, maintained separately. Rivet works without it.

**Development snapshot:** bounded P1–P3 profiles are implemented with synthetic-provider tests. No live model, multi-day endurance, browser or native desktop qualification is claimed. The current executable is tied to its source checkout; this is not a portable release or general installation guide.

## Try the source checkout

Requires the existing Linux build environment described in [building](docs/building.md): Rust 1.99.1, a C compiler, bubblewrap and cached locked dependencies. From this repository root:

```bash
scripts/cargo-contained offline build --locked --offline
.build/target/debug/rivet --help
.build/target/debug/rivet config check --json
.build/target/debug/rivet status --json
```

Without configuration, provider status is `absent`. These inspections do not download models or create runtime state. See the [observed output and checked examples](docs/solo.md). For a configured, permission-reviewed coding task, follow [Solo](docs/solo.md) and [providers](docs/providers.md).

## What works today

| Capability | Evidence and limits |
| --- | --- |
| Solo coding | Reviewed inference and file/process/Git proposals; synthetic coding fixtures, no live model validation. |
| Recovery and bounded autonomy | Durable intents, artifacts, exact reconciliation, pause/stop/resume and explicit detach; bounded crash/stress tests, no reboot or endurance claim. |
| Pair, Crew, Fleet | Explicit reviewed task graphs, fenced owners, separate contexts and evidence-bound review. |
| Shared Model, Concurrent, Relay | Declared capacity/instance/KV reservations; synthetic streams, manual external-runtime activation. |
| Cookbook | Read-only qualified llmfit JSON import and whole-allocation recommendations. No model acquisition or benchmark. |
| Mesh and Adaptive | Local deterministic peer claims and permitted quiescent transitions; no distributed cluster or unrestricted spawning. |
| Providers, MCP and skills | Compatible Chat Completions, native Ollama/Anthropic profiles, scoped stdio MCP and pinned skills; synthetic conformance only. |

See [verification](planning/research/post-p3-verification.md) for checks reproduced in this run and [mode details](docs/modes.md) for cardinalities.

## Documentation

- [Documentation index](docs/README.md): build, first use, configuration and troubleshooting.
- [Security and permissions](docs/security.md): boundaries, reviews and recovery limits.
- [Contributing](docs/contributing.md): checks and review expectations.
- [Roadmap](docs/roadmap.md): implemented profiles and future qualification.
- [Planning index](planning/README.md): design history, decisions and research.

## Versioning and releases

Rivet follows [Semantic Versioning 2.0.0](https://semver.org/). See the [versioning policy](docs/versioning.md), [planned v0.1.0-alpha.1 notes](docs/releases/v0.1.0-alpha.1.md) and [release-readiness checklist](docs/release-readiness.md). The pre-release is in preparation and is not yet ready for publication.

## License

Rivet is licensed under the **GNU General Public License v3.0 only** (`GPL-3.0-only`). See [LICENSE](LICENSE) for the full terms.

Third-party components retain their respective licenses. Rivet is provided without warranty, as described in the license.
