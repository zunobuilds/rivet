# Quick start and Solo

First [build](building.md), then inspect configuration without a provider or model:

```bash
.build/target/debug/rivet config check --json
.build/target/debug/rivet status --json
.build/target/debug/rivet chat --help
```

Observed in this checkout without configuration/state:

```json
{"execution_available":false,"provider":"absent","schema_version":1,"valid":true}
```

The JSON above is the actual `config check --json` result, not an inference-readiness certificate. Status additionally reported `state: "missing"` and `adapters_available: true`. These commands work without inference credentials. To create local durable state explicitly:

```bash
.build/target/debug/rivet state init
.build/target/debug/rivet session new
.build/target/debug/rivet session list --json
```

State creation writes `.rivet-state/`; it is optional for inspection and does not configure a provider. An inference run needs an existing provider with an explicit context ceiling; see [provider setup](providers.md). With that configured and its destination/data use authorized, the current Solo entry point is:

```bash
.build/target/debug/rivet chat --task 'Inspect a selected source file and propose a small fix' --steps 10 --deadline-seconds 600 --token-budget 16384
```

This example follows the checked CLI help; it was not executed against a model. Before each inference or tool operation, read the exact scope and enter `approve <review-digest>` only for the displayed request. Deny unwanted requests. A provider's success text is not proof that a file changed or a check passed; review recorded receipts and tested source references.

Use `chat --session UUID --task TEXT` for a new bounded run in an existing session. `UUID` and `TEXT` are placeholders. Uncertain prior effects require repair first. Continuation uses provenance references, not a saved complete transcript.

For an active run, `run status UUID`, `run pause UUID` and `run stop UUID` expose current controls. `run resume UUID` previews the durable snapshot; resuming requires its exact `--approve DIGEST` and fresh operation reviews. Unknown inference reservations are not refunded or resent by resume. Run `run --help` and `repair --help` before recovery actions.

Detached execution is explicit through `chat --detach` or the hierarchy equivalent; it needs the qualified temporary core unit. Closing an ordinary attached terminal is not the future universal detach/reconnect UX. Global sessions, slash commands and Safe Quit are [research](../planning/research/post-p3-product-ui.md).

## Check an example without making an inference request

```bash
.build/target/debug/rivet config check --config docs/examples/model-free.toml --json
.build/target/debug/rivet config check --config docs/examples/local-compatible.toml --json
```

The first reports `provider: "absent"`; the second reports `provider: "configured-unverified"`. Neither contacts the example address. Both commands were executed successfully during documentation validation. Copy/adapt the [local example](examples/local-compatible.toml) only after selecting an existing authorized provider. Never enter a real key into this file.

## Inspect a hierarchy before deciding to run it

This writes a JSON plan inside existing ignored build storage, with no inference or state creation:

```bash
.build/target/debug/rivet run template --mode pair --task 'Review a bounded change' > .build/example-plan.json
.build/target/debug/rivet run hierarchy --plan .build/example-plan.json
```

The second command returns `executed: false` and a `review_digest`. It does not approve the plan. Read task scopes and paths before any later execution; a task description is not filesystem authority. The full reviewed coding/check fixture is described in [P1c evidence](../planning/research/p1c-solo.md), and its native rerun remains outside this run's authority.

Next: [mode constraints](modes.md), [run controls and recovery](operations.md), or [troubleshooting](troubleshooting.md).
