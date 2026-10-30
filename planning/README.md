# Rivet planning index

**Current status: P1a foundation implemented in `176bbb77`, with cleanup committed as `090f47c`. P1b tool APIs pass the patch/check/diff/commit fixture; P1c reviewed Solo coding and P1d native profiles/MCP/skills pass synthetic and native fixtures.** The owner has authorized an extended run through P3c in dependency order. This permission waives stage approval/stopping gates only for this run; installations, host changes, live inference, outside-project access and pushes remain forbidden. Completion requires actual stage evidence, not authorization alone.

Current evidence: [P1a implementation](research/p1a-evidence.md) and [extended-run checkpoint/handoff](research/extended-run.md). P2a recovery and P2b bounded lifecycle are qualified; P3a–P3c have not passed acceptance; the file-foundation history is in the extended-run handoff and the newly permitted delegation route is in [P1b runtime evidence](research/p1b-runtime.md). P0.5/P0.6 documents below remain design/history references, not implementation claims. No new push is authorized; historical user-reported pushes do not grant one.

The owner has replaced the mandatory 72-hour gate with the [bounded P2–P3 contract](P2_P3_QUALIFICATION.md); historical evidence below does not reinstate that gate.

## Current implementation handoff

| Stage | Current result |
| --- | --- |
| P1a cleanup | Committed `090f47c`; recovery/cancellation/metadata regressions pass. Rustfmt/Clippy unavailable. |
| P1b | Bounded tool APIs qualified on this host: descriptor-safe files, cgroup/namespace/Landlock checks/cancellation, selected local Git, patch/check/diff/receipt fixture. See runtime evidence for limits. |
| P1c | Compatible streamed transport, bounded context/budgets, reviewed terminal Solo loop, continuation and source recipe; synthetic/native evidence in [P1c](research/p1c-solo.md). No live inference tested. |
| P1d | Synthetic Ollama/Anthropic, scoped rmcp stdio and content-pinned skills; [evidence](research/p1d-adapters.md). No live inference/keyring qualification. |
| P2a | Schema 4 durable intents/artifacts, explicit reconciliation, fenced bundle restore and bounded crash/full-storage/native-worker tests; [acceptance evidence](research/p2a-acceptance.md). |
| P2b | Schema 5 durable deadlines/checkpoints/control/resume and explicit same-supervisor detach; [bounded lifecycle evidence](research/p2b-lifecycle.md). Optional endurance is deferred to P7. |
| P3a–P3c | Unstarted; require accepted P2 and preceding P3 steps. Approved mode cardinalities unchanged. |

Current next work: implement and qualify P3a–P3c in order, preserving the accepted P2 boundaries. The owner has authorized these stages for this run; prerequisite checks still apply. The temporary user-systemd qualification route remains narrowly authorized; no installation or general outside-project access is implied.

## Implementation entry points

| Document | Purpose and authority |
| --- | --- |
| [Project brief](RIVET_PROJECT_BRIEF.md) | Approved product constraints; substantive requirements remain preserved. |
| [Master implementation plan](MASTER_IMPLEMENTATION_PLAN.md) | Current final technical recommendations: selected stack/pins, contracts, modules, D01–D15 defaults, security and exact P1–P7 task order. |
| [P1 implementation handoff](P1_IMPLEMENTATION_HANDOFF.md) | First task/environment gate, P1a–P1d sequence and the complete future P1a Codex prompt. Do not execute without explicit owner approval. |
| [Decision dispositions](open-decisions.md) | Current selected recommendations, alternatives, owner decisions and verification gates for every D01–D15. |
| [Implementation roadmap](implementation-roadmap.md) | Stage dependencies, deliverables, risks and acceptance rationale; master task IDs govern exact work. |

## Supporting research and provenance

| Document | Purpose |
| --- | --- |
| [Architecture](architecture.md) | Component/interface/ledger/extension rationale from P0.5; explicit current choices live in the master. |
| [Agent execution](agent-execution.md) | Approved counts and ownership/messaging/context/resource rationale; P0.6 finalizes Mesh/Adaptive policy recommendations. |
| [Security and recovery](security-and-recovery.md) | Threat boundaries, effect classes, crash matrix and future qualification requirements. |
| [Technical research](technical-research.md) | Preserved technology/framework comparisons, duplication audit and neutral license comparison. |
| [Source register](research/source-register.md) | Historical sources plus P0.6 registry/release/license/compatibility verification supplement. |
| [Research log](research/research-log.md) | Findings, failures/discrepancies, checkpoints, publication context and actual validation results by stage. |

**Approved** means a product/work constraint from the user, [AGENTS.md](../AGENTS.md) or brief, not implementation. **Selected recommendation** means P0.6 has chosen a concrete technical default for adoption by a future stage approval. **Owner decision** means research cannot supply permission or choose policy, notably Rivet's license. **Verification gate** means an actual build/runtime/security check still has to pass; public metadata is not a successful test. Neither a dependency table nor a prompt in a file grants permission to install, code, call models, access desktop/devices, push or publish.

## Continuing from this checkpoint

1. Read the [repository rules](../AGENTS.md), [documentation guide](../GITHUB_DOCUMENTATION_GUIDE.md), brief, master, handoff and latest log; inspect local Git status/history. The [root README](../README.md) remains the public-facing product overview.
2. Do not start another general architecture study. Execute only an explicitly approved bounded stage. Change a selected recommendation only for an evidenced incompatibility/security issue, measured failure or explicit owner decision, and preserve why it changed.
3. Reuse the qualified repository-local `scripts/cargo-contained` route for source fetching and offline checks. Keep Cargo ancestor/home configuration hidden. Source acquisition, tool installation and model/live-provider calls are different permissions. Read the latest extended-run evidence before choosing the next task.
4. Keep every file/cache/artifact within Rivet unless a later instruction explicitly authorizes a specific exception. Root `/.research-local/` is ignored raw scratch retained locally. Substantive evidence, conclusions, alternatives, decisions and source metadata remain tracked in planning documents.
5. Preserve historical research; append source access dates/version/license evidence and meaningful log entries. Validate links, decisions, secret exposure and exact diff. A documentation-only check is not an application test.
6. Follow usage checkpoints and the current owner-approved stage sequence. Commit meaningful milestones and report failed/unrun checks. During the extended run, proceed only after prerequisites pass; stop on genuine blockers or usage thresholds. Outside that run, separate stage approvals/stops still apply. No push is authorized.
