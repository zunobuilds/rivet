# Quick start and Solo

First [build](building.md), then inspect configuration without a provider or model:

```bash
.build/target/debug/rivet config check --json
.build/target/debug/rivet status --json
.build/target/debug/rivet chat --help
```

These commands work without inference credentials. To create local durable state explicitly:

```bash
.build/target/debug/rivet state init
.build/target/debug/rivet session new
.build/target/debug/rivet session list --json
```

State creation writes `.rivet-state/`; it is optional for inspection and does not configure a provider. An inference run needs an existing provider with an explicit context ceiling; see [provider setup](providers.md). With that configured and its destination/data use authorized, the current Solo entry point is:

```bash
.build/target/debug/rivet chat --task 'Inspect a selected source file and propose a small fix' --steps 10 --deadline-seconds 600 --token-budget 16384
```

This example was parser-checked, not executed against a model. Before each inference or tool operation, read the exact scope and enter `approve <review-digest>` only for the displayed request. Deny unwanted requests. A provider's success text is not proof that a file changed or a check passed; review recorded receipts and tested source references.

Use `chat --session UUID --task TEXT` for a new bounded run in an existing session. `UUID` and `TEXT` are placeholders. Uncertain prior effects require repair first. Continuation uses provenance references, not a saved complete transcript.

For an active run, `run status UUID`, `run pause UUID` and `run stop UUID` expose current controls. `run resume UUID` previews the durable snapshot; resuming requires its exact `--approve DIGEST` and fresh operation reviews. Unknown inference reservations are not refunded or resent by resume. Run `run --help` and `repair --help` before recovery actions.

Detached execution is explicit through `chat --detach` or the hierarchy equivalent; it needs the qualified temporary core unit. Closing an ordinary attached terminal is not the future universal detach/reconnect UX. Global sessions, slash commands and Safe Quit are [research](../planning/research/post-p3-product-ui.md).
