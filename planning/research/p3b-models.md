# P3b model policies and Cookbook qualification

30 October 2026. Prerequisite P3a: `7ae9911`. No live model, paid inference, benchmark, installation or model/runtime acquisition was used.

## Implemented bounded profile

Schema 7 stores model policies, actual-instance identities, request/KV reservations, unknown resource holds, relay phases and evidence handoffs in the existing ledger. `run hierarchy --execution PATH` adds an explicit JSON policy to the admission digest. Policy-file edits cannot alter an active run. Resume loads the stored policy, preserves aggregate budgets, fences prior ownership and retains uncertain resources. A conflicting old-run hold prevents a new pool for that instance/destination.

Shared Model uses distinct contexts and counts resident weights once per declared actual instance; per-request KV is separate. One slot is the default. Increasing slots requires an explicit owner capacity declaration bound to the exact profile digest and an evidence hash/statement, displayed in admission. **This is an owner assertion, not an independently verified Rivet measurement.** Untrusted llmfit data/model output cannot create it. Unknown context/capacity or contradictory instance identities fail closed. RAM/VRAM bounds include declared OS, tool/core, browser and loading overlap reserves. Reservations enforce the declared bounds; they are not an OS monitor or a guarantee about an externally managed model process.

Concurrent execution pipelines independent provider quanta through the same scheduler, with one in-flight request per actor, instance slot limits, writer leases and one aggregate budget. Pool and inference-budget reservations commit atomically before send. Cancellation drains the owned futures; unknown usage/resource reservations remain held. Coordinators must consume updated child evidence before integration, even when their earlier inference finished after a child.

Relay phases are explicit task-index/profile selections with serial requests, drained boundaries and exact ledger-artifact handoffs. Contexts are reconstructed at a phase change; hidden model state is never copied. Existing resident profiles can switch after an exact phase review. An unloaded target records a pending activation and **stops without invoking a runtime or inference**. `run pool UUID --observation PATH` previews an exact owner-observed completion; `--approve DIGEST` records it. The strict observation identifies the request or activation, profile digest, evidence hash and owner statement, with the relevant complete/unloaded/resident flags. Only explicit observed completion can release a hold/finish activation. No automatic load/unload adapter, download or fallback is claimed. This is a deliberate manual boundary for external runtimes whose control APIs are not qualified.

`run pool UUID` is model-free inspection (active status includes cached pool data). Request reconciliation releases only the resource hold: unknown inference budget remains reserved/charged and requires its own P2 disposition. Invalid review digests or incomplete observations cannot clear a hold.

A contract gap was corrected: inference now requires `context_tokens` in the explicitly selected provider configuration. Missing ceilings remain syntactically valid for model-free configuration review but are unsupported for inference. Permits bind that ceiling; conservative byte-based input plus output admission must fit. Existing synthetic fixtures declare 65536. This is a minimal fail-closed compatibility correction to the master's known/user-declared-context requirement, not a tokenizer or measured model-capacity claim.

## Cookbook and source provenance

`cookbook --input PATH --version 1.1.17 --plan PATH --allocations PATH --preference efficient|balanced|ambitious` imports a bounded existing llmfit CLI JSON export and ranks explicit whole-hierarchy policies. Input bounds: 64 KiB, 128 model rows, 32 allocation alternatives. Only the qualified `system`/`models` shape and required fit fields are used. Installed/tool-use/runtime/context conditions and conservative instance/KV/loading/headroom checks apply to the complete allocation. Efficient prioritizes memory headroom, Balanced balances minimum role fit score/headroom, Ambitious prioritizes mean imported score. Reasons and source/allocation hashes are returned; imported estimates are never labeled Rivet measurements. Unsupported alternatives remain explicit. Acquisition/benchmark/verify-command fields are inert data and never executed. No duplicate model catalog, optimizer service or automatic discovery exists.

Bounded source-only acquisition inspected the planned **1.1.17** serializers (registry also listed 1.1.18; it was not adopted). No source was compiled/executed or added to Cargo dependencies:

| Registry source | Bytes | SHA-256 | License |
| --- | ---: | --- | --- |
| [llmfit-core 1.1.17](https://crates.io/crates/llmfit-core/1.1.17) | 1,459,720 | `0ba69ba49acf701186fa58f2736a42a48b50ceb5a2ca99066e468e93ceb14578` | MIT |
| [llmfit 1.1.17](https://crates.io/crates/llmfit/1.1.17) | 218,090 | `c96f02f0d76ce0637914bd1c54787624573e8e5d1301d337db66f5e6f12e6eb6` | MIT |

The source serializers in `src/fit.rs`, `src/models.rs`, `src/display.rs` and `src/serve_shared.rs` establish the CLI envelope/fields. Archive checksums match public registry metadata; this is integrity against that channel, not independent producer authentication. Raw metadata/archives/selected sources stay inside ignored `.build/research-llmfit/`. Cargo's 227 locked dependency sources/features remain unchanged; bundled SQLite remains 3.54.1.

## Actual checks and limits

- Deterministic tests cover shared-weight/KV accounting, slots, per-actor ownership, modified/missing qualification, unknown context, memory exhaustion, loading overlap, activation review/observation, serial phase fencing and unknown holds.
- Actual synthetic HTTP streams demonstrate default shared serialization, two simultaneously accepted requests under the explicit synthetic capacity declaration, and three serial relay phases using two already resident synthetic profiles. Contexts remain separate; all requests share one budget and settle. These are endpoint fixtures, not model-speed/quality results.
- Provider EOF retains both usage and resource uncertainty; exact owner observation clears only the resource hold without refunding tokens. Exhausting the parent request ceiling rolls back the pool reservation before a second send. Unknown/insufficient context cannot authorize inference.
- Read-only llmfit-shape import/ranking tests cover all three priorities, whole-allocation failure and ignored command/acquisition fields. No llmfit executable was run.
- Contained offline all-target and current-host native checks pass. Rustfmt/Clippy remain unavailable. P2 crash/restore/control/stress and P3a hierarchy/review tests continue passing after migration.

Physical capacity/residency declarations are trusted owner inputs; Rivet did not measure hardware or exercise real inference. External-runtime activation is explicitly observed/manual, not automatic. No guarantee against malicious same-user host software, actual reboot/power loss or unqualified platforms is claimed. P3c Mesh/Adaptive remains next.
