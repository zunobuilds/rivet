# Rivet planning index

**Stage P0.5 — architecture and feasibility research.** This planning set documents research and proposed engineering work. Rivet remains unimplemented: no Cargo scaffold, application, installer, build, runtime test or benchmark is present. This stage authorizes public read-only research, repository-scoped documentation and local commits only. No push, remote change or publication is authorized.

## Read in this order

| Document | Purpose and authority |
| --- | --- |
| [Project brief](RIVET_PROJECT_BRIEF.md) | Approved product requirements; first-README-only task scope retired. Preserve substantive requirements. |
| [Architecture](architecture.md) | Proposed Rust components, contracts, ledger/session boundaries, failure handling and Rivet-side Unrivet API. |
| [Agent execution](agent-execution.md) | Approved role counts; proposed ownership, scheduling, messaging, context and resource mechanics. |
| [Security and recovery](security-and-recovery.md) | Proposed threat model, permission/isolation boundaries, crash windows, reconciliation and autonomy lifecycle. |
| [Technical research](technical-research.md) | Technology/framework/provider comparisons, duplication audit and license alternatives without selecting a license. |
| [Implementation roadmap](implementation-roadmap.md) | Proposed P1–P7 increments, dependencies, deliverables, risks, future checks and acceptance gates. |
| [Open decisions](open-decisions.md) | Requirement traceability, unresolved decisions D01–D15, rejected/deferred options and evidence gaps. |
| [Source register](research/source-register.md) | Upstream URLs, publishers, observed versions/licenses, access date and supported conclusions. |
| [Research log](research/research-log.md) | Meaningful research steps, source disagreements/failures, checkpoints and validation limits. |

## How to interpret status

**Approved** means a product/work constraint from the user, [AGENTS.md](../AGENTS.md) or brief. It does not mean implemented. **Proposed** means this research recommends a design or candidate; it needs future approval and evidence. **Unresolved** means an owner choice or technical qualification remains. **Verified upstream observation** means only that the cited source supports the stated upstream fact; it does not demonstrate Rivet behavior.

**P0.5 research deliverables are complete; documentation checks passed as recorded in the research log.** P1 and every later implementation stage remain **not started and not authorized**. No dependency pins, product license or platform support promises are adopted by these documents. The [root overview](../README.md) remains the product entry point; the planning set is the detailed research record.

## Continuing research

1. Read [repository instructions](../AGENTS.md), the [documentation guide](../GITHUB_DOCUMENTATION_GUIDE.md), the brief and the log/checkpoint before changing this set. Inspect local Git status/history and preserve unrelated work.
2. Work only inside the authorized Rivet directory. Raw outputs/caches/scratch belong in root `/.research-local/`, are retained locally and ignored. Evidence summaries, comparisons, limitations, decisions and source metadata belong in tracked planning documents.
3. Start with an open decision; consult primary upstream docs, exact release manifests/licenses and advisories. Record access date and version/snapshot, distinguish observed facts from inferences, and preserve failed checks or conflicting sources. Never copy entire third-party sources into these documents.
4. Keep one authoritative explanation per topic and link to it; update the source register and meaningful research log entries alongside findings. Do not promote a recommendation to approved status without the user's decision.
5. Validate local links/anchors, consistency, ignored scratch versus tracked substantive evidence, whitespace and secrets. Commit only the authorized documentation; report unverified claims/checks honestly.
6. Follow usage checkpoints in AGENTS.md. Before any implementation, installation, prototype, benchmark, paid call, external publication or next stage, obtain explicit authorization for that stage and its effects. Finish the approved stage, save a durable checkpoint, report and **STOP**.
