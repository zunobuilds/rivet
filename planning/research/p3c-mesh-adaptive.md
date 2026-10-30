# P3c local Mesh and bounded Adaptive qualification

30 October 2026. Prerequisite P3b: `6e6b67d`. This completes the authorized P2–P3 sequence for the bounded profiles documented here. No P4 implementation, real-model inference or distributed service was added.

## Mesh

`run template --mode mesh --task TEXT` defaults to two Independent peers; `--peers N` accepts 1–32. Minimal still permits only Solo. The same graph, broker, registry, ledger, budgets and worker isolation apply. Ready tasks retain FIFO/round-robin scheduling. The supervisor selects a deterministic eligible peer by outstanding ownership count and stable actor-ID tie-break, then retains that single task owner across quanta. Per-quantum fences invalidate old tokens; role identity is separate from request slots. The designated root Independent routes integration/conflicts to the main chat without acquiring Lead permissions. There is no network cluster, peer-elected authority or second scheduler.

Known workspace conflicts stop affected Mesh work, record exact operation references and wait for main-chat input. `run status` exposes the graph/conflict; explicit digest-bound `run resume` is required to unblock it. Cached proposals are discarded, fresh inference/tool review is required, and the failed write is never replayed. Ambiguous effects still require P2 repair/disposition; a lease expiry or a peer assertion never proves the old effect absent.

## Adaptive

Adaptive is an explicit controller in the execution policy's optional `adaptive` member. The reviewed plan chooses the initial concrete mode; RunSpec records that concrete composition and subsequent versioned transitions. Bare `Mode::Adaptive` without a concrete policy is unsupported. Example policy fragment for a Pair plan:

```json
{
  "initial": "pair",
  "allowed": ["solo", "pair"],
  "agent_ceiling": 2,
  "coordination": false,
  "review": false,
  "director": false,
  "tokens_per_actor": 1,
  "steps_per_actor": 1,
  "cost_per_actor": 0,
  "max_transitions": 1,
  "role_profiles": [],
  "mesh_peers": 2
}
```

These are explicit planning estimates, not tokenization/pricing measurements; actual reservations retain their stricter P2/P3b bounds. The policy is part of the full execution/admission digest. No allowed list or an ambiguous requirement keeps the current mode. Otherwise selection chooses the smallest permitted valid composition fitting declared resource estimates and remaining aggregate token/step/cost bounds; ties prefer the current mode. Director/review/coordination requirements and all existing task/review criteria are preserved. A contradictory initial requirement is rejected. Unknown target allocations are unsupported rather than guessed; one explicitly configured common profile can serve all roles.

Transitions require a completed planning boundary, no active lease/request/activation, no unresolved effects and no pending messages. Actors that already performed non-coordination work cannot be retired or silently reassigned; existing technical evidence keeps its owner/provenance. Specialist-bearing compositions retain their mode unless an explicit transfer is supported; no extra specialists are invented. Required task definitions, paths, dependencies, criteria and IDs remain unchanged. Shapes that cannot represent a permitted composition safely are rejected rather than adding unreviewed tasks. Queue order is retained.

Schema 8 preserves the prior full graph and policy digest in bounded mode history. Current task ownership, active roster, RunSpec version, permission epoch, routing and model assignments change atomically. Historical actor/operation rows remain intact. Old grants/forwarding consents are revoked; contexts are reconstructed from trusted scope and ledger evidence. The same run/session/deadline/consumed steps and budget remain. No hidden model-state copying or automatic acquisition occurs.

`run adaptive UUID --allow MODE ...` previews a **narrowing** of the stored allowed list; `--approve DIGEST` records it under exclusive quiescent ownership. Supplying no modes disables automatic transitions. This command cannot expand permissions; broader policy requires a new reviewed run. Pausing before administration avoids racing active work.

## Actual evidence

- Mesh peer-count/Minimal validation, deterministic claims, stable ownership across 1000 yield/claim quanta, stale/wrong-actor token rejection and common FIFO/round-robin tests pass. P3a's ten-seed 1000-quantum fairness matrix remains passing.
- Actual synthetic Mesh execution completes with two Independent actors, one shared budget and settled requests. A separate two-writer fixture preserves the first file, pauses the conflicting task for main chat, rejects unblocking without explicit resume and proves that resume itself performs no write/replay.
- Adaptive tests cover smallest valid composition, immutable task/path/criterion preservation, quiescence rejection, ambiguous requirements, revoked lists, insufficient budget, coordination requirements and preservation of Crew's required technical review when reducing Fleet.
- An actual synthetic Adaptive run first uses a Lead in the user-selected Pair composition, then changes to Solo at the planning boundary. Subsequent requests use the Independent context. Exactly three inference requests share the original budget; two RunSpec versions and one prior-graph history record remain. Narrowing the completed run's allowed list to empty succeeds; re-expansion is denied.
- The final `scripts/cargo-contained offline test --locked --offline --all-targets` suite passes after schema 8 migration: 77 passed, zero failed, eight native tests skipped in containment and all eight passed separately through `python3 scripts/test-native.py`. Source checks pass. Logs remain repository-local in `.build/final-tests.txt` and `.build/p3c-native.txt`. The prior Solo-only registry assertion was updated for now-valid Mesh; the initial failure is not represented as a pass. A final parallel run observed a transient Busy in post-shutdown inspection; the test now retries only Busy for at most 500 ms, with production locking unchanged. Formatter/Clippy remain unavailable. Dependencies/features are unchanged; bundled SQLite is 3.54.1.

## Scope limits / handoff

These are explicit bounded graphs and owner-declared capacity profiles with synthetic-provider qualification. Adaptive does not invent task trees, weaken review, transfer unspecified skills, retire actors with completed coding/review ownership or silently choose a new endpoint. Model activation remains explicit/manual as documented in P3b. Actual multi-day endurance, reboot/power loss, physical model fit, live service/model quality and broader platform certification remain untested; optional endurance belongs to P7.

The authorized P2a/P2b/P3a/P3b/P3c work is complete in these profiles. Future work requires a fresh authorized scope; the next master stage is P4a (browser), **not started**. Do not automatically install Playwright/Chromium, models or runtimes, access desktop/Unrivet, push, change remotes or edit AGENTS.md. Preserve local commits and read current evidence before future changes.

Final usage checkpoint: `codex-status` returned 7% five-hour allowance remaining. No new implementation batch was started; the final evidence/commit closes this run. No push was performed.
