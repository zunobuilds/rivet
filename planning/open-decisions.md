# Open decisions and requirements audit

**P0.6 final dispositions.** Technical defaults are now selected recommendations for implementation, not owner approval or implemented behavior. The [master plan](MASTER_IMPLEMENTATION_PLAN.md) owns exact pins, modules, interfaces and work packages; the [P1 handoff](P1_IMPLEMENTATION_HANDOFF.md) owns the bounded next prompt. The [brief](RIVET_PROJECT_BRIEF.md) remains the approved product reference; [architecture](architecture.md), [agent execution](agent-execution.md), [security/recovery](security-and-recovery.md) and [technical research](technical-research.md) contain proposals. The [roadmap](implementation-roadmap.md) identifies future evidence gates.

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

## D01–D15 final dispositions

IDs are local planning identifiers, not the brief's historical question numbering. **R** means a selected technical recommendation, ready for adoption by the named stage approval; **O** means an owner decision remains. **V** means an implementation verification gate, never evidence already obtained. No broad architecture study remains; later failures trigger narrow evidenced corrections.

| ID | Disposition and selected default | Alternative/risk and evidence | Remaining owner/verification gate |
| --- | --- | --- | --- |
| D01 | R: one Rust 2024 package/bin/lib, stable/minimum 1.99.1 initially; Tokio/clap line chat, no TUI framework | Ratatui/Crossterm remain possible later if terminal needs justify them; older compiler support untested. S01/S03/S57 | P1a approval and existing toolchain/build boundary; V: actual lock/features/build. |
| D02 | R: direct reqwest/rustls adapters; first llama.cpp-compatible Chat Completions profile; config provider unset | Rig reuse loses to API/scheduler coupling for initial scope; Ollama/Anthropic adapters in P1d. S17/S23–S25/S56/S60 | P1c scope approval; real endpoint/model/data/spend are O per run, never selected by preset. |
| D03 | R: project-local .rivet-state, one ledger/supervisor, rusqlite worker, WAL/FULL/foreign keys, retained std lock; bundled engine constraint | SQLx extra stack rejected; shared network ledger unsupported. Published metadata resolves old wrapper discrepancy but not actual engine. S56/S58/S59 | P1a scope; V: SQLite >=3.54.1, lock/rollback/filesystem/migration tests; outside state location needs explicit grant. |
| D04 | R: default deny semantic broker with exact grants, root capabilities, bubblewrap and strict required Landlock rights | Prompt-only/command-prefix authority rejected; no fallback to unsupported enforcement. S13/S22/S29/S62 | P1b worker/sandbox acquisition/privilege approval; V: escape/revocation/process-tree cases; P1a executes no production tool. |
| D05 | R: workers no network; trusted provider adapter has scoped endpoint grant; later one constrained egress proxy | Landlock alone cannot enforce hostname policy; raw RE traffic is separate. S22/S29/S30/S60 | V: proxy/DNS/redirect/loopback/IPv6/UDP/fd bypass matrix before general egress; O for data/destination grants. |
| D06 | R: stable intent/receipt IDs, compare-and-set transitions, unknown/reconciliation, verified blobs, backup-before-migrate | No universal exactly-once promise; outbox cannot make remote effects transactional. S27/S28/S49 | P2a approval; V: every crash window, actual fsync/restore behavior and stale-child fencing. |
| D07 | R: single-owner local Mesh with deterministic claims; Adaptive smallest permitted valid mode at quiescence, explicit allowed list/ceiling | Distributed consensus deferred; specialist assignments cannot violate counts; user decisions require recipient/scope consent. Master specifies scheduler and routing | P3 approvals adopt these policies; V: conflict/fairness/transition invariants. No further design-default choice blocks P1. |
| D08 | R: Node 24.22.2 LTS + JS Playwright 1.64.1 stdio worker and matched Chromium | Python viable but loses to official JS runner/API simplicity; no Rust wrapper/direct protocol reimplementation. S34/S35/S61 | P4a explicit runtime/browser acquisition; V: browser revision, sandbox/egress/artifacts on target OS. |
| D09 | R: atspi 0.30.0 + ashpd 0.13.13, compatible zbus 5.x worker, separate tree/screen/input grants; GNOME/KDE Wayland fixture matrix | C libatspi alternative not selected; latest zbus 6 incompatible with chosen ranges; broad desktop grants need target guards. S36/S39/S63 | O: desktop access/target matrix at P4b; V: native GTK/Qt, compositor/toolkit coverage, focus/revocation. |
| D10 | R: explicit 60-min/100-step initial bounds, one model slot, positive remote spend ceiling, keyring or ephemeral credential input; required bounded fault/stress qualification at P2b; optional endurance at P7 | Initial pause-on-disconnect, later explicit detached same supervisor; optional transcripts off/7-day opt-in; no auto deletion of required evidence. Master defines numeric targets | O: run limits/data retention/long-run and credential permissions; V: cancellation/soak/usage uncertainty. No encrypted/tamper-proof claim. |
| D11 | R: namespaced JSON-RPC v1 stdio manifest/lifecycle; common registry/broker/ledger; rmcp for MCP client, not extension governance | Native ABI and duplicated runtime rejected. Registry rmcp Apache-2.0 declaration does not erase retained notices. S21/S22/S56 | P5 scope; O: any sibling access; V: fake extension then approved bilateral version/disable/evidence tests. |
| D12 | R: aqua/uv, explicit immutable paths, no lazy/PATH fallback; Mike Farah yq identity; preserve accepted tool families/presets | No new package manager or blind all-109 install. Full inventory missing; exact per-tool artifact fixtures still required. S40/S41/S54/S55 | O: missing inventory/preset membership and acquisition at P5/P6; V: root-contained caches/archives/versions/licenses. Not a P1a blocker. |
| D13 | O: no Rivet license chosen; preserve neutral comparison | MIT/Apache dual or single permissive versus MPL/GPL/AGPL/LGPL reciprocity are owner policy choices, not technical defaults. S43–S48 | Explicit owner selection before licensing/distribution claims. Private P1a can omit license field/file; no invented grant. |
| D14 | R: original upstream, available trusted verification, staging/smoke/drain/atomic activation/rollback; use existing verification tools; TUF reference for own release trust | A local hash alone is not authenticity; absent trustworthy verification requires reduced-assurance tool-specific owner decision. S40/S42 | O: keys/trust roots/unsigned exceptions/signing operations at P6; V: expiry/rotation/rollback/transformed-artifact tests. No custom verifier service. |
| D15 | R: initial x86_64 Linux qualification targets, all approved final capabilities before mature Linux v1.0; macOS native+Playwright then Windows | Test-target recommendation is not support evidence; exact images/backend versions recorded at P4/P7. S65; master acceptance matrix | O: release/support/security contacts/name/trademark/publication; V: full P1–P7 matrix. No dates or release authorization. |

**Immediate approval blockers:** P1a authorization; a compliant existing build route or a separately scoped automatic Cargo-config-read exception; permission for source-only registry acquisition if dependencies are not already available within Rivet. No host settings were inspected. No license, model, live provider, desktop, Unrivet or release approval is necessary to implement the strictly bounded P1a path. No such implementation permission exists in P0.6.

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

No prototype, benchmark, sandbox exercise, live inference, application/desktop test, dependency lock, ABI check, footprint measurement, signing deployment or cross-repository compatibility test was run in either planning stage. P0.6 resolved metadata edges/defaults; runtime claims remain gated. Provider retention/pricing, specialist catalog maintenance/licenses, complete transitive license compatibility and release signing operations are not established. Upstream version/license snapshots need artifact-specific revalidation before use.

The initial commit `6fb21b0` establishes documentation and a local history baseline, not software progress. The brief's old local-development anecdote is historical and no longer describes the inspected Git history; it is retained because this task permits only retiring its first-task instructions, not rewriting its substantive content. Root README guidance is preserved unchanged. See the [research log](research/research-log.md) for access failures, discrepancies and final checks.


## Post-P3 decisions and outstanding approvals

This section supersedes conflicting historical D01/D11/D13 and blanket dashboard exclusions above for future planning; it does not change existing code.

| ID | Classification | Disposition / gate |
| --- | --- | --- |
| PP01 | Approved product | Solo + Simple default; optional native hierarchies, chat-first hybrid UI and one global Command Center across authorized projects/sessions. Commands/shortcuts/layout are proposals. |
| PP02 | Approved product | Workspace model C, stable renameable logical groups plus folder discovery; groups are not permission boundaries. |
| PP03 | Approved product | Recipe/config + pinned references default export, selected complete archive optional. |
| PP04 | Approved product direction | Official optional Unrivet first; no initial third-party loading; disabled features operationally absent with bytes/settings preserved. Rivet-Bench optional official mod planned. |
| PP05 | Research direction | One logical integrated Shared Resource Controller; local resources only, remote server resources excluded; safe recovery, deterministic predictor first. Algorithms/metrics/protocol need implementation scope approval. |
| PP06 | Recommendation, not approved | Independent Git worktrees/branches for concurrent writers, lease/CAS integration and selective inverse diffs. Existing Git lane rejects linked worktrees; future implementation must qualify them explicitly. |
| PP07 | Recommendation, not approved | AOSP Emulator + AOSP image for app testing; Cuttlefish for platform/headless fallback, Waydroid optional trusted desktop path. Artifact/host/device acquisition gates remain. |
| PP08 | Recommendation, not selected | GPL-3.0-only for Rivet and compatible official mod code. Owner must select exact license/scope and contribution terms; no LICENSE created. |
| PP09 | Open owner policy | Recycle Bin retention/caps/privacy, protected-evidence cleanup rules; no fixed duration chosen. |
| PP10 | Open semantics | Duo session vocabulary versus implemented Pair mode; do not rename/redefine without decision. |
| PP11 | Open implementation scope | Multi-session project ownership, global IPC/owner lifecycle, model switching/retry policy, tool update policies and exact safe disable/drain behavior. |
| PP12 | Verification gate | Fresh native P1–P3 worker/Git/Solo/MCP/recovery/detach/hierarchy checks need narrow outside-root host authority; publication remains blocked. |

Detailed proposals and risk/acceptance gates: [post-P3 handoff](POST_P3_HANDOFF.md). No new broad implementation permission follows from this register.
