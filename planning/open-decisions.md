# Open decisions and requirements audit

**P0.5 research only.** No technical recommendation below is an implementation approval. The [brief](RIVET_PROJECT_BRIEF.md) remains the approved product reference; [architecture](architecture.md), [agent execution](agent-execution.md), [security/recovery](security-and-recovery.md) and [technical research](technical-research.md) contain proposals. The [roadmap](implementation-roadmap.md) identifies future evidence gates.

## Approved requirements to preserve

| Requirement | Authoritative planning coverage | Audit outcome |
| --- | --- | --- |
| Standalone Rust-oriented terminal coding platform; Unrivet optional | [Architecture](architecture.md) | Preserved. No sibling access, embedded file manager, OpenCode runtime or separate Messenger/Git UI proposed. |
| Solo/Pair/Crew/Fleet exact counts; Mesh/Adaptive constrained design | [Agent execution](agent-execution.md) | Preserved; roles differ from model instances and specialist skill assignments. Mesh semantics remain open. |
| One supervisor, broker, durable task/message/audit ledger and registry | [Architecture](architecture.md) | Preserved; proposed logical modules do not imply a service per feature. SQLite remains principal candidate. |
| Concurrent/Shared Model/Model Relay; multi-day authorized work | [Agent execution](agent-execution.md), [recovery](security-and-recovery.md) | Preserved as goals; no speed, exactly-once or unattended-lifecycle guarantee invented. |
| Cookbook, llmfit reuse, Efficient/Balanced/Ambitious | [Technical research](technical-research.md) | Preserved; broad automatic optimization deferred; benchmarks only by explicit opt-in. |
| No default weights or inference-runtime install in any preset; model-free administration | [Roadmap P1/P6](implementation-roadmap.md) | Preserved. No missing-provider cloud fallback, upload or paid call. |
| Playwright/Chromium and qualified native Linux inspection owned by Rivet | [Technical research](technical-research.md) | Preserved. Browser and native evidence are separate; Rust supervisor may use a qualified foreign-language browser worker. |
| Independent presets; Portable recommended, System opt-in, no Hybrid | [Roadmap P6](implementation-roadmap.md) | Preserved. Full is not all inventory entries; precise membership remains unqualified. |
| Accepted tool adapters and approved deduplication strategy | [Inventory audit](technical-research.md) | Preserved without fabricating the missing 109-entry manifest or per-tool qualification. |
| Shared immutable assets, authorized verified updates and rollback | [Technical research](technical-research.md) | Preserved. Compression experiments stay off; no measured savings claimed. |
| Per-project Unrivet Yes/No; reuse runtime/tools under separate grants; original evidence | [Architecture](architecture.md), [recovery](security-and-recovery.md) | Preserved. API details are Rivet-side proposals, not bilateral compatibility claims. |
| Useful pre-1.0, mature Linux-only v1.0, macOS with native inspection then Windows | [Roadmap](implementation-roadmap.md) | Preserved with no dates or invented release milestones. |
| Stage approval, evidence, durable checkpoint, then stop | [Planning index](README.md) | Applies to every future implementation stage. This stage permits documentation/local commits only. |

## Decisions requiring owner approval or technical evidence

All IDs are local P0.5 decision identifiers, not the historical brief's question numbering. None is marked decided by this research.

| ID | Decision and current recommendation | Alternatives/tradeoff | Evidence and gate |
| --- | --- | --- | --- |
| D01 | Rust dependency/MSRV/terminal baseline: Tokio + clap; optional Crossterm/Ratatui rendering | Minimal chat first versus full-screen terminal rendering; compiler support follows actual selected features | S01/S03/S10/S11; pin versions/features/license/MSRV and verify at P1a. |
| D02 | First provider and abstraction: narrow Rivet contract; compare Rig provider reuse against direct reqwest | Rig reduces adapter work but has API churn; direct adapters require more protocol maintenance. Select a concrete first profile without making it mandatory for users | S06/S17/S23–S25; approved endpoint/data/cost scope and synthetic conformance at P1a/P1c. |
| D03 | Ledger scope/location/schema/build: one project ledger, one supervisor, rusqlite worker preferred | SQLx async toolkit; application-managed state versus user-chosen local path; bundled engine versus system update cadence | S04/S05/S27/S28; local-filesystem constraints, backup, schema migration and actual SQLite version at P1a/P2a. |
| D04 | Permission/grant/sandbox policy: semantic broker, fail closed on missing required enforcement | Session-scoped versus operation-scoped approvals; stronger isolated workers versus compatibility; no blanket shell-prefix trust | S13/S22/S29–S31/S33; exact allowed mounts, hooks/helpers, grants and revocation at P1b. |
| D05 | Network enforcement: deny by default; brokered destination-aware egress for granted uses | No-network profiles, proxy mediation or stronger runtime; raw device/network research must remain separately permissioned | S22/S29/S30/S35; DNS/redirect/loopback/IPv6/UDP/proxy/inherited-socket bypass fixtures before network capability claims. |
| D06 | Recovery/effects: intent/receipt/reconciliation, stable IDs, unknown-outcome state | Per-operation conditional retries; remote idempotency only where proven; never universal exactly-once | S02/S27/S28/S49; durability/synchronous policy, artifact flush/rename ordering, process identities, restore and crash windows at P2a. |
| D07 | Mesh/Adaptive ownership/arbitration, mode transitions and decision routing | Local task claims/fencing preferred; distributed consensus deferred. Define consent scope for important user-decision forwarding and conflict escalation | S18 reference plus approved counts; executable acceptance cases before P3c, routing policy before P3a. |
| D08 | Playwright worker binding/runtime: official JS/TS or Python, choose one | JS/TS test runner ecosystem versus Python/uv acquisition reuse; community Rust wrappers remain unqualified | S34/S35/S41; exact runtime/browser/OS assets, notices, containment and cancellation at P4a. |
| D09 | Linux native support and permissions: AT-SPI + separately qualified portal operations | Rust atspi/zbus versus C libatspi; GNOME/KDE/Wayland/X11/toolkit matrix and broad session grants | S36–S39; per-target authorization and actual native fixtures at P4b; API version is not installed desktop evidence. |
| D10 | Autonomy, resource targets, credentials and retention | Pause-on-disconnect initially; explicit detached mode later; keyring versus explicit ephemeral credentials; balance audit retention with privacy | S12/S26/S32/S49/S50; approve lifecycle, cancellation/recovery/soak thresholds, uncertainty reserve and data retention before P2b/P3b. Encryption/tamper-evidence scope remains unchosen. |
| D11 | Extension protocol/versioning: bounded namespaced RPC with MCP reuse where useful | Native dynamic ABI deferred; MCP transport alone insufficient for consent/install/evidence lifecycle | S21/S22; negotiated major/minor compatibility, frame/output bounds, shared registry access and synthetic extension at P5; cross-repo access needs separate approval. |
| D12 | Exact tool inventory/presets/acquisition: qualify aqua + uv, avoid unused assets | Reuse verified installs consistent with location; authorized managed assets or absent. Clarify yq identity and missing inventory entries | S40/S41/S54/S55 plus brief; per-tool version/license/provenance/format/footprint before P5/P6; no bulk installation implied. |
| D13 | Rivet FOSS license: no selection recommended without owner priorities | MIT, Apache-2.0, dual MIT/Apache, MPL, GPL/AGPL, LGPL tradeoffs in technical research | S43–S48; owner choice and actual dependency/plugin distribution review before public licensing/distribution claims. |
| D14 | Update trust roots/signing/unsigned policy and rollback | Reuse upstream attestations/checksums and aqua; consider TUF where trust gap justifies it; never invent upstream signatures | S40/S42; key rotation, expiry/freeze/rollback handling and transformed-artifact lineage at P6. |
| D15 | Linux release/support matrix, release evidence, name/trademark and later platforms | Narrow qualified matrix first; no implied all-Linux or cross-platform support | Roadmap P7; thresholds set before tests, notices and actual support/security route before release; publication separately authorized. |

## Rejected and deferred options

| Option | Disposition and rationale | Revisit condition |
| --- | --- | --- |
| Model bundles, mandatory Qwen, automatic inference runtime/cloud fallback | Rejected by approved requirements | Explicit product requirement change; never inferred from research. |
| Embedding OpenCode or a file manager/Messenger/dashboard | Outside approved boundary | Explicit new scope; not a routine implementation choice. |
| AutoGen as new foundation | Rejected recommendation: upstream README says archived; older contradictory text retained in log | Credible maintained successor/fork would require a new source and API/license audit. |
| Full Rig runtime plus Rivet controller plus LangGraph persistence | Avoid duplication of scheduling/state/permissions | Show a single authoritative state owner and a concrete reuse benefit at a future design gate. |
| Independent schedulers/message stores/audit services per feature | Rejected complexity for current single-host design | Measured lifecycle/privilege need, while preserving one authoritative ledger/runtime. |
| Distributed Mesh cluster and consensus service | Deferred, not implied by peer mode | Explicit distributed requirement and failure/authority design. |
| Native dynamic Rust extension ABI | Deferred due to compatibility/isolation coupling | A stable interface and concrete in-process requirement outweigh process RPC. |
| Direct browser-protocol reimplementation/unqualified Rust binding | Deferred; official worker preferred | Demonstrated maintainability/coverage/security benefit. |
| gVisor/VM as universal default | Deferred alternative; stronger isolation but compatibility/packaging burden | Approved high-risk workload qualification and measured operational impact. |
| Global vector DB, broad auto-optimizer, benchmark daemon | Deferred/no demonstrated need | Scoped user value, privacy/resource design and separate benchmark approval. |
| UPX/compressed SquashFS runtime storage | Experimental, disabled by approved default | Explicit experiment and independent portability/startup/memory/security/update evidence. |

## Material evidence gaps

No prototype, benchmark, sandbox exercise, live inference, application/desktop test, dependency lock, ABI check, footprint measurement, signing deployment or cross-repository compatibility test was run. Provider retention/pricing, specialist catalog maintenance/licenses, complete transitive license compatibility and release signing operations are not established. Upstream version/license snapshots need artifact-specific revalidation before use.

The initial commit `6fb21b0` establishes documentation and a local history baseline, not software progress. The brief's old local-development anecdote is historical and no longer describes the inspected Git history; it is retained because this task permits only retiring its first-task instructions, not rewriting its substantive content. Root README guidance is preserved unchanged. See the [research log](research/research-log.md) for access failures, discrepancies and final checks.
