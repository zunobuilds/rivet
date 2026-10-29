# Proposed implementation and qualification roadmap

**Status: P0.5 documentation only.** All stages below are future proposals requiring separate explicit approval of scope and side effects. No source, scaffold, build, application test, installation, model download or benchmark was produced during this research. P1–P7 retain the [brief's](RIVET_PROJECT_BRIEF.md) sequence; lettered subdivisions make approval scopes smaller, not new release promises.

Every stage must end with evidence-backed results, failed/unrun checks, a durable local checkpoint and a stop. An acceptance criterion is a future requirement, not a checked result. Any network acquisition, live provider use, paid call, desktop/device access, benchmark, push or publication must be within that stage's explicit authorization. Approval of architecture or this roadmap is not implementation approval.

## Dependency map

| Stage | Depends on | Useful outcome |
| --- | --- | --- |
| P1a–P1c Solo foundation | Owner resolves P1-blocking decisions in the [decision register](open-decisions.md) | A bounded, permission-controlled Solo coding workflow and model-free administration. |
| P2a–P2b Recovery | P1 operation/grant IDs, receipts and provider boundaries | Crash reconciliation, restart and qualified multi-day runs. |
| P3a–P3c Hierarchies | P2 ownership/message persistence and budget accounting | Pair/Crew/Fleet first, then qualified Shared Model/Relay, Mesh/Adaptive. |
| P4a Browser | P1 sandbox/broker, P2 cancellation/artifacts; P3 not technically required for initial browser adapter | Qualified Playwright/Chromium workflows. |
| P4b Native Linux | P4 permission/artifact conventions, P2 recovery; separate desktop qualification | Consent-driven semantic native inspection and separately granted screen/input. |
| P5 Extension boundary | P2/P3 common runtime, P4 tool contracts where reused | Standalone Rivet plus optional project-enabled Unrivet. |
| P6 Packaging/updating | Stable capability manifests from earlier stages | Qualified presets, Portable assets, verified transactions and rollback. |
| P7 Linux release qualification | All advertised P1–P6 capabilities and explicit remaining decisions | Mature Linux-only v1.0 qualification evidence; no date committed. |

Security and recovery are cross-cutting: P1 needs safe cancellation and effect identity before P2 can recover it. P6 expands acquisition/updating; it is not permission to defer basic provenance or secret handling until after tool execution exists. Browser and native work may be scheduled independently where their dependencies permit, but each still needs its own stage approval.

## P1: functional Solo coding agent

### P1a — executable shell, administration and contracts

**Dependencies:** decide Rust/MSRV/dependency policy, minimal terminal interaction, first provider contract, project-state location and minimum sandbox profile (D01–D06). No default provider/model may be preselected for inference.

**Deliverables:** an approved Rust scaffold; CLI help/config review/status/repair functions that work without model credentials; versioned session/run/operation envelopes; exactly one Independent; explicit provider-not-configured status; one broker/registry/ledger boundary even if the initial schema is small. Installation acquisition is a separate explicitly approved side effect, not implicit in selecting crates.

**Future checks:** invalid/valid Solo composition; malformed config/schema; absence of provider; no network/model acquisition during administration; restricted project roots and denied execution; terminal restoration; synthetic provider events and cancellation.

**Acceptance:** administrative flows work offline without inference credentials; no tool can bypass the broker; missing capabilities are honest errors; no claim of multi-day recovery. Report exact versions/features/license evidence and minimum supported environment. Risk: early convenience defaults could embed hidden cloud calls or broaden filesystem access.

### P1b — bounded file, terminal and Git tools

**Dependencies:** P1a grants and effect/receipt contracts; approved Linux isolation profile and workspace mutation policy.

**Deliverables:** read/search/edit/patch/create/rename/delete operations with reviewable diffs, expected-revision checks and artifact/output bounds; supervised terminal/build/test/lint/debug commands; Git inspection and approved local mutation lane. Initial tools should use existing qualified executables where possible; no bespoke package manager. Each operation records intent and receipt.

**Future checks:** traversal/symlink/hard-link/special-file fixtures, concurrent human edits, partial writes and interrupted commands; Git hooks/helper execution denied or specifically approved; output flood, orphan descendants and cancellation; network-denied command execution; source and lockfiles preserved. Tests use synthetic repositories, not unrelated user projects.

**Acceptance:** an allowed patch and configured check produce evidence; denied operations have no authorized dispatch; stale patches fail without overwriting edits; child cleanup and error reporting are demonstrated. Remote Git operations remain unavailable unless separately authorized and implemented. Risk: treating test/build commands as intrinsically safe.

### P1c — first useful end-to-end coding flow

**Dependencies:** P1b plus a qualified provider adapter; live inference requires a separately permitted endpoint, context/data scope and cost ceiling.

**Deliverables:** understand a small project, plan, edit, invoke relevant checks, show diff and report evidence; session continuation and bounded context; simple new-project creation; initial MCP/skills registration policy without untrusted auto-execution. Minimal remains Solo only.

**Future checks:** deterministic fake-provider coding scenarios; malformed/partial/duplicate tool calls; refusal/approval routes, provider error/timeout/quota, context overflow, interrupted usage accounting and no unapproved fallback. Optional live smoke check only within approved data/spend.

**Acceptance:** demonstrate a useful coding task with exact workspace/result references and a reproducible test record; model-free administration still works; no invented installation instructions or release claims. Risk: one provider's success being generalized to all compatible endpoints.

## P2: durable recovery and multi-day autonomy

### P2a — transactions, replay and reconciliation

**Dependencies:** P1 stable operation IDs and receipt/effect classes; choose schema, SQLite engine/build, backup and durability policy (D03/D06). Require an engine containing the documented WAL-reset fix, verified independently of the wrapper.

**Deliverables:** task/message/audit records in one ledger; compare-and-set transitions, supervisor lock and fencing; durable operation intents and deduplication; outcome-unknown reconciliation; artifact staging/verification; versioned migrations, consistent backup and restore; resumable context with provenance. Follow the authoritative crash-window matrix in [security and recovery](security-and-recovery.md).

**Future checks:** terminate at every recorded crash window; duplicate/out-of-order receipts/messages; power-loss simulation where approved; disk-full, busy database, corrupt/missing artifacts, unknown newer schema, backup restore, clock changes, expired grants and stale workers.

**Acceptance:** no ambiguous non-idempotent effect is blindly repeated; task/message/audit transitions remain consistent; restore drill recovers a documented usable checkpoint; failed recovery stops affected dispatch. Risk: confusing ledger atomicity with atomic shell/network effects. No exactly-once marketing claim.

### P2b — bounded long-running execution

**Dependencies:** P2a and approved disconnect/resume policy, retention, budgets and measurable cancellation/recovery targets (D04/D06/D10).

**Deliverables:** pause/stop/status, out-of-band cancellation, run deadlines and budgets, credential/approval expiry handling, restart summary, retry limits and explicit blocked/waiting states. Initial disconnect policy may pause; a later approved detached mode must reuse the same supervisor and ledger.

**Future checks:** an explicitly approved multi-day soak duration; suspend/reboot, provider outage, quota exhaustion, UI loss, user absence, disk pressure, keyring lock and cancellation under saturation. Record actual elapsed time, resource peaks, recovery points and unknown effects; fake providers first, model benchmarks only by opt-in.

**Acceptance:** measured recovery/cancellation and budget behavior meet targets set before testing; no silent grant renewal, hidden cloud fallback or unbounded retry. Multi-day support is claimed only for the tested lifecycle/profile. Risk: long authorization being interpreted as permission to change task scope.

## P3: hierarchy execution and resource feasibility

### P3a — Pair, Crew and Fleet

**Dependencies:** P2 durable ownership/messages, workspace leases and aggregate budget reservations.

**Deliverables:** exact approved role cardinalities, task graph and single-owner assignments, leader/director routing, Reviewer evidence bound to revisions, acknowledgments, specialist invariants and consent records for forwarding important user decisions. Reuse one registry/broker/runtime.

**Future checks:** all legal and illegal role configurations; parent/child failure, dependency cycles, duplicate deliveries, reviewer crash/disagreement, human edits, expired leases with old process still alive, cancellation and task-owner transfer.

**Acceptance:** every tested task has one accountable current owner; technical review cannot grant human permissions; no successful parent with required unresolved child/unknown effect; Minimal cannot enable hierarchies. Risk: role prompts replacing enforceable ownership.

### P3b — Concurrent, Shared Model, Model Relay and Cookbook

**Dependencies:** P3a; provider capacity profiles; approved llmfit data ingestion and resource estimation policy.

**Deliverables:** bounded endpoint/model slots, separate agent contexts, serial queues and relay evidence handoffs; transparent Efficient/Balanced/Ambitious recommendations; hierarchy-wide RAM/VRAM/context/cost estimates with uncertainty. No mandatory model and no additional model catalog/optimizer service.

**Future checks:** deterministic capacity/queue scenarios, starvation and cancellation, shared-parent budget exhaustion, loading-overlap estimates, context separation, unsupported endpoints and missing hardware data. Actual resource/model benchmarks require separate opt-in and recorded hardware/model/runtime versions.

**Acceptance:** explain why a hierarchy fits, queues or is unsupported; configured budgets constrain all descendants; no claimed speed/memory savings without measurement. Risk: counting shared weights once but forgetting per-request KV-cache and peak loading memory.

### P3c — Mesh and Adaptive

**Dependencies:** D07 approved ownership/arbitration/transition rules and P3a/P3b invariants.

**Deliverables:** configurable peers with explicit task ownership; permitted Adaptive mode selection; quiescence, fenced reassignment and routing continuity during mode changes. No distributed cluster infrastructure unless independently justified and approved.

**Future checks:** competing claims, tie resolution, mode transition during running/unknown work, specialist limits, invalid count changes, unfair scheduling and policy/budget refusal.

**Acceptance:** all transitions preserve the chosen mode's count rules and current permissions; impossible transitions produce an explicit request/denial; peer failure does not silently lose ownership. Risk: “adaptive” becoming unbounded agent spawning. Mesh/Adaptive cannot be advertised as complete while their semantics remain open.

## P4: browser and qualified Linux native inspection

### P4a — Playwright and Chromium

**Dependencies:** choose one official binding/runtime and supported browser/OS profile (D08); reuse sandbox, broker, artifact and cancellation contracts.

**Deliverables:** bounded process adapter for browser actions, origin/data-transfer permissions, separate browser profile/context, screenshot/trace artifacts, cleanup and cancellation. Chromium assets/dependencies are authorized acquisitions with notices; no default browser download in model-free administration.

**Future checks:** local fixture web app automation, selectors/waits, downloads/uploads, redirects, subresources/WebSockets/service workers, malicious page instructions, credential redaction, browser crash and lingering processes. Test Chromium sandbox compatibility rather than disabling it silently.

**Acceptance:** reproducible browser workflow in the qualified environment with bounded egress/artifacts and denied-origin evidence; no native desktop-support claim inferred from it. Risk: treating browser contexts or route interception as complete host/network isolation.

### P4b — semantic native Linux inspection

**Dependencies:** D09 target matrix and permission policy; qualified atspi/zbus or libatspi path; explicit desktop-session approval.

**Deliverables:** application/window identification, bounded accessibility-tree inspection, capability/status reporting; separately consented screenshot/input/clipboard through qualified portals/native APIs. Use actual native fixtures, not browser-only examples.

**Future checks:** approved GNOME/KDE Wayland and any X11 targets, multiple toolkits, missing accessibility nodes, focus/window changes, consent denial/revocation, portal close, locked screen, headless session and app crash. Verify non-target apps cannot receive intended target actions within the supported profile.

**Acceptance:** publish a precise tested matrix and known gaps; read/input/screen capabilities are distinguished; unsupported operations fail honestly. Risk: session-wide accessibility/portal access exceeding per-app product promises.

## P5: optional Unrivet extension boundary

**Dependencies:** stable common registry, broker, ledger and hierarchy contracts; D11 API/manifest/version policy. Work in Unrivet or cross-repository testing needs separate explicit access authorization; this roadmap does not grant it.

**Deliverables:** namespaced tool manifests, protocol negotiation, version rejection, scoped RPC workers, evidence receipts, editable project opt-in and uninstall/disable behavior. Without Unrivet, new-project flow remains ordinary Rivet; with it installed, ask the approved Yes/No question. No automatic device/network grants.

**Future checks:** synthetic extension first; missing/incompatible/malformed extension, tool-name collision, permission bypass attempts, restart/EOF/cancellation, schema change, immutable evidence hashes and standalone Rivet after uninstall. Later authorized cross-repo contract/format fixtures cover accepted tools and YARA-X/extraction fallback compatibility.

**Acceptance:** extension cannot duplicate or replace core runtime ownership, widen hierarchy/permissions, or break ordinary Rivet when removed; original evidence preserved. Risk: inventing specialist inventory membership or claiming external integration without actual compatibility tests.

## P6: presets, Portable assets and updates

**Dependencies:** qualified capability manifests and owner decisions on acquisition/trust, license, notices and support matrix (D12–D15).

**Deliverables:** independent Rivet/Unrivet preset manifests; Portable recommended/System opt-in/no Hybrid; verified existing-install reuse; authorized aqua/uv acquisition with contained paths and lazy/download policies; immutable shared assets with per-project grants; staged verify/smoke-test/atomic activation/rollback receipts. No tool/model bundles simply to fill Full.

**Future checks:** all preset/location combinations; no weights/inference-runtime acquisition by default; no downloads from status/config commands; relocation/offline/proxy failure; malicious archives/symlinks/path traversal; checksum/signature failure, unsigned policy, wrong ABI/dependency, interrupted activation, concurrent users of old version, rollback and disk exhaustion. Separate compatibility/consent tests for rules/databases/models/plugins.

**Acceptance:** tool-specific origin/license/version/trust/footprint receipts resolve; no active task's executable changes underneath it; failed update leaves last qualified version usable; no writes outside approved installation/data locations in tested profiles. Report actual archive/installed/runtime/peak sizes independently. UPX/compressed SquashFS remain disabled unless separately approved experiments pass their own tests. Risk: a package manager's default cache/install behavior violating Portable and consent boundaries.

## P7: Linux v1.0 qualification

**Dependencies:** all advertised features accepted, unresolved release-blocking decisions closed, owner-selected license/notices and verified security/support routes. A feature omission needs an explicit scope decision; do not redefine the final vision silently.

**Deliverables:** supported Linux distribution/architecture/kernel/desktop matrix; reproducible source build/install/uninstall and upgrade/rollback procedures; provider/tool conformance profiles; security/recovery/soak evidence; dependency/license/advisory inventory; user documentation from real commands and expected results; release checklist and known limitations.

**Future checks:** complete offline and no-model paths; all role counts, modes, execution policies and preset boundaries; crash/restore and multi-day scenarios; browser/native fixtures; Unrivet absence/removal and authorized compatibility tests; secret/egress/injection tests; migration/update/uninstall preserving user data; terminal accessibility and error usability. Set performance, resource, cancellation, recovery and soak thresholds before qualification and record hardware/configurations. Paid/model benchmarks remain opt-in.

**Acceptance:** every public capability claim has reproducible evidence on the stated matrix; critical security/recovery failures block release; licensing/trust decisions have accountable approval; no release/publication without separate authorization. Linux-only v1.0 follows useful pre-1.0 milestones, not the first prototype.

## After Linux v1.0

macOS follows with both Playwright and actual macOS native application inspection; Windows follows later. Each needs its own permission, sandbox, credential, packaging, update and native inspection qualification. No dates, exact versions or implemented platform support are claimed. Broader automatic Cookbook optimization remains deferred and requires its own value/complexity review.
