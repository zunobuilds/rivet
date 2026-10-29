# Rivet planning index

**P0.6 — final pre-implementation planning.** Rivet still has no application, Cargo scaffold, installer or tested runtime. The blueprint and ready-to-paste P1a request are complete; validation is recorded in the research log. Their presence is not permission to execute them. P1 and every later implementation stage remain **not started and not authorized**.

P0.5 was completed in `759a327`. The user subsequently reported pushing it to origin/main; the locally recorded tracking reference agrees. P0.5 is therefore no longer described as unpublished. **That user push is not authorization for any agent push.** P0.6 permits repository-scoped documentation, read-only public research and local commits only; no push, remote/GitHub change or publication is authorized. No remote visibility verification was performed in P0.6.

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
3. First P1a task is environment/authority preflight before source edits. Cargo's ancestor-config discovery needs a compliant existing contained route or separately explicit narrow permission; source acquisition, tool installation and model/live-provider calls are different permissions.
4. Keep every file/cache/artifact within Rivet unless a later instruction explicitly authorizes a specific exception. Root `/.research-local/` is ignored raw scratch retained locally. Substantive evidence, conclusions, alternatives, decisions and source metadata remain tracked in planning documents.
5. Preserve historical research; append source access dates/version/license evidence and meaningful log entries. Validate links, decisions, secret exposure and exact diff. A documentation-only check is not an application test.
6. Follow usage checkpoints. Finish the approved stage, save a local commit/checkpoint, report failed/unrun checks and **STOP**. No next stage or push without its own explicit authorization.
