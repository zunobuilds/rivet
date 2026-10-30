# P1 implementation handoff

**Current authority update (post-P3):** P1–P3 bounded profiles are implemented; [fresh verification](research/post-p3-verification.md) is partial because native host checks were not rerun. P0 statements below about absent implementation/authorization are historical. The [post-P3 brief](POST_P3_RESEARCH_BRIEF.md) supersedes older exclusions of hybrid dashboard/global coordination, approves workspace model C and recipe-first export, and defines official-mod/resource/memory/recovery research boundaries. See [current handoff](POST_P3_HANDOFF.md) and [decisions](open-decisions.md). No P4+ implementation or license adoption is authorized by these plans.


**P0.6 deliverable; no P1 authorization.** This is the execution handoff for the [master blueprint](MASTER_IMPLEMENTATION_PLAN.md), not a further research request. Its future commands/interfaces do not exist yet. Only a later explicit user instruction can activate the prompt below. No code, manifest, test or dependency was created/executed while writing this handoff.

## P1 sequence and first task

| Stage | Bounded outcome | Gate to next stage |
| --- | --- | --- |
| P1a | Rust executable, model-free administration, minimal SQLite/session state, broker/registry/provider contracts and inert contract tests | Local evidence/commit; STOP for P1b approval. |
| P1b | Scoped file tools, qualified process sandbox/cancellation, permitted local Git, fixture checks and diffs | Demonstrate safe authorized mutations; STOP for P1c approval. |
| P1c | First configured provider adapter and useful Solo coding loop, context/budgets/session continuation/new-project recipe | Synthetic end-to-end coding evidence; optional live check only under separate endpoint/data/cost grant; STOP. |
| P1d | Native additional provider profiles, rmcp client and explicit skills/MCP policy | Conformance/security evidence; STOP for P2 approval. |

**First task is P1a.1 environment/authority preflight, before source edits.** Verify an already available Rust 1.99.1 toolchain, C compiler and contained build route without installing or inspecting unrelated paths. Do not silently lower the compiler baseline, switch SQLite engines, widen permissions or add dependencies to get a check to pass.

Cargo's documented ancestor-config discovery is a real boundary issue. Preferred route: an already available owner-approved contained runner that prevents reads of parent/home Cargo configuration, keeps source/cache/temp/output within Rivet, and runs dependency build scripts without network/host credentials. Do not invent or prototype such a runner inside P1a. Alternative: the owner separately authorizes narrowly enumerated automatic ancestor configuration reads by Cargo; the prompt below does **not** grant that exception. If neither route exists, report the exact missing capability/permission and stop at preflight. Ordinary intrinsic reads by authorized installed executables remain governed by AGENTS.md; manual outside-project inspection is never implied.

A compatible dependency graph, actual SQLite >=3.54.1 and available sandbox/build tools remain implementation acceptance checks. This is not permission to install them. The complete 109-ID tool catalog, later desktop/signing policy and Rivet's license choice do not block a private P1a foundation; no license field/file or public distribution claim may be invented.

## Ready-to-paste P1a request

The following complete request is for the owner to send in a **future turn only after approving its scope**. Retain every boundary and stop condition when copying. It authorizes only P1a and specifically distinguishes source-only dependency acquisition from installing binaries/models. If source acquisition or the preflight route is not acceptable, the owner must revise that permission before sending it; P0.6 itself grants none of these actions.

```text
Implement Rivet P1a ONLY: the bounded Solo foundation defined by planning/MASTER_IMPLEMENTATION_PLAN.md and planning/P1_IMPLEMENTATION_HANDOFF.md. This instruction authorizes P1a source/manifest/test/documentation work and local commits, not P1b or a full coding agent.

Work only in /home/zuno/Projects/rivet-unrivet/rivet/. Read AGENTS.md, GITHUB_DOCUMENTATION_GUIDE.md, the approved brief, master plan, decision register and this handoff. Inspect local Git status/history and preserve unrelated work. Follow usage checkpoints. Do not edit AGENTS.md or the documentation guide, inspect Unrivet, push, change remotes or publish anything.

0. PREFLIGHT BEFORE SOURCE EDITS
Use only an already available Rust 1.99.1 toolchain/C compiler and owner-approved contained build route. Do not install/update tools or download toolchain binaries. All generated data, dependency caches, test files, temporary directories and build artifacts must stay inside this repository. Use repository-local CARGO_HOME, CARGO_TARGET_DIR and TMPDIR; do not repurpose HOME or CODEX_HOME.
Cargo may search ancestor configuration even with CARGO_HOME set. This request does NOT authorize reading parent/home configuration or unrelated files. Require an existing contained route preventing that discovery, or an independently explicit owner authorization for the exact automatic config reads. Do not invent/install a runner. If the toolchain, C compiler or compliant execution route is unavailable, report the concrete blocker and STOP before source edits. Do not treat a missing tool as permission to install it.

I authorize read-only public registry metadata plus source-only acquisition of the approved Rust dependencies and their resolved transitive source dependencies from crates.io/static.crates.io, into the repository-local cache, solely for P1a. No binary/model/runtime downloads, git-source dependencies, private registry credentials, paid services, global caches or host changes. Separate this fetch phase from offline compilation/testing; build scripts must run without network or host credentials. If acquisition cannot meet that boundary, stop and report rather than widening access.

1. FILE SCOPE AND STACK
Create one Rust 2024 Cargo package named rivet, with one binary and a testable library, initial rust-version 1.99.1. Do not create a multi-package workspace, rust-toolchain auto-installer, graphical UI, daemon or empty future-feature directories. Do not select a license or add a LICENSE file/license field. Commit Cargo.lock.
Allowed changes: Cargo.toml, Cargo.lock; active P1a source modules listed below; meaningful unit/integration fixtures under tests/; .gitignore entries for repository-local state/cache/temp artifacts; truthful root README setup/status changes; a concise tracked P1a evidence/checkpoint entry under planning/research/. No unrelated files.
Use exactly the master P1a candidates: tokio 1.54.1, tokio-util 0.7.20, clap 4.6.7, serde 1.0.229, serde_json 1.0.154, toml 1.1.8+spec-1.1.0 (require =1.1.8), rusqlite 0.41.1, libsqlite3-sys 0.39.1 constraint, cap-std 4.0.3 for state/config paths, thiserror 2.0.21, uuid 1.29.0, tempfile 3.27.0 for tests. Use the master's minimal features/default-feature policy. Do not add HTTP, model, keyring, MCP, browser, desktop, framework, sandbox or updater dependencies in P1a.
Review exact registry checksums, features and transitive compiler/license constraints while resolving Cargo.lock. The planning matrix proves metadata availability, not a successful build. If an actual incompatibility/security issue appears, save the evidence and propose the smallest correction; do not silently substitute a stack. Verify the bundled engine with SELECT sqlite_version(); require >=3.54.1. Do not use a system SQLite fallback or patch vendor source to hide a failure.

2. IMPLEMENT ACTIVE P1a MODULES
Create main.rs/lib.rs, cli.rs, config.rs, domain.rs, error.rs, supervisor.rs, permissions.rs, tools/mod.rs, providers/mod.rs, ledger/mod.rs and the used migration/session modules. Create only modules with implemented behavior/tests.
Provide versioned IDs/envelopes and typed errors; RunSpec, ProviderPort events, ToolDescriptor/Proposal, BrokerDecision/Grant, ExecutionReceipt and LedgerCommand as defined in the master. The fake provider/inert tool are test support only; no production tool executor or real provider call exists in this stage.
Exactly one Independent is valid for the executable Solo path. Recognize the planned mode names but reject unsupported execution modes clearly, without redefining their approved counts. Do not implement hierarchies, Shared Model/Relay or specialist spawning in P1a.

3. PROPOSED ADMINISTRATIVE CLI v1
Implement --help and --version, config check [--config PATH] [--json], status [--state-dir PATH] [--json], state init [--state-dir PATH], session new [--state-dir PATH], and session list [--state-dir PATH] [--json]. Paths in this task must resolve inside the repository or its contained fixtures. An explicit path is not a boundary exception.
Default product paths are .rivet.toml and .rivet-state/ under the chosen project root. Never discover Rivet config in parent/home directories or search for credentials. Unknown configuration fields/schema versions fail with sanitized diagnostics. Default provider is unset; administration works without weights/keys/endpoint. A syntactically configured provider is not inference-ready: status must distinguish absent/configured-unverified and report real adapters/execution unavailable in P1a.
Read-only help/config/status/session-list must not create files or initialize a database. Missing state is an honest state, not an installer/repair trigger. Only explicit state init/session new may create/update state. Use meaningful stable exit classes: 0 success, 2 invalid input/config, 3 unsupported/not-ready capability when requested, 4 state/lock/I/O failure, 130 user interruption. Missing provider in an administrative status report is not itself an error.
No hidden default model, download, cloud fallback, telemetry, account setup, network health check or paid call.

4. STATE AND AUTHORITY
Implement the master's minimal schema v1, local exclusive retained lock, owner epoch and transactional state+audit transitions on a dedicated bounded ledger thread. WAL, synchronous FULL, foreign keys on every connection, 5-second busy bound. No DB transaction across provider/tool work. Use cap-std for descriptor-root config/state access from this stage; retain the validated state-directory identity across SQLite opening, reject links/unsafe paths before DB open. Initial state is project-local; owner-only permissions, reject escaping/symlinked state roots, refuse unknown newer schema and lock conflicts. For standalone status/list, use the existing lock without creating it: report busy for active ownership and recovery-required for WAL/recovery remnants without opening the DB; inspect only a quiescent verified DB with read-only immutable URI under that lock, never against live writers. Prove no DB/WAL/SHM creation or mutation in those paths. Do not claim crash-safe multi-day recovery yet.
Broker denies by default. Grants bind actor/operation/proposal digest/resources/epoch/expiry; stale/mutated requests cannot reuse approval. Registry rejects duplicate namespaced IDs/versions. Only the supervisor dispatches the inert test adapter after recorded intent; test provider callbacks cannot invoke it directly. Critical ledger failure prevents affected dispatch. Test-only fake provider stops at proposals unless supervisor/broker admit the inert effect.
Cancellation uses a separate control path so a waiting fake provider or saturated ordinary queue cannot swallow stop. Implement cancellation of owned in-memory tasks and clean ledger shutdown. No shell child/process-tree enforcement is claimed before P1b.

5. REQUIRED EVIDENCE
Use deterministic unit/integration tests with explicit temp parents inside the repository, no host temp or credentials. Cover config unknown/malformed/schema errors; no implicit config discovery; help/status/config/list non-creation and no network construction; missing provider remains usable administration; exact Solo rule/unsupported mode; duplicate registry IDs; stale/revoked/mutated grants; fake provider cannot bypass broker; intent before inert dispatch; atomic rollback of state+event on injected DB failure; lock conflict/unknown schema; redacted errors; cancellation while fake waits or normal queue is full; SQLite engine requirement.
After the authorized fetch/lock phase, run cargo fmt --all -- --check, cargo clippy --locked --offline --all-targets -- -D warnings, and cargo test --locked --offline --all-targets inside the compliant build route. Exercise the actual administrative CLI against a contained fixture and verify before/after directory contents and structured output. Do not install a missing fmt/clippy component; report a failed or unrun check accurately. No live inference, application-under-development build beyond Rivet's own approved foundation checks, browser/desktop test or benchmark.
Use real test seams and observable effects; do not write brittle tests that merely duplicate strings or private implementation structure. Do not add CI publication or network services.

6. FINISH AND STOP
Review scope, secret exposure, permission enforcement and documentation accuracy. Inspect Git status and the complete diff; run git diff --check. Stage only P1a files and make a descriptive ordinary local commit on main; never rewrite history. Report files/modules, exact resolved versions/engine, actual commands/results, failed/unrun checks, remaining limitations and commit hash. Root README must describe P1a accurately and leave all later capabilities planned.
STOP. Do not implement P1b/P1c/P1d/P2 or push without a new explicit approval.
```

## Approvals and failure handoff

Sending the prompt is approval for P1a only, including its explicitly bounded source-acquisition permission. It does not authorize the separate outside-config exception, tool installation, live model session, or distribution license. A preflight failure should report the missing compiler/runner or exact extra read scope needed; it must not become another broad design exercise. A build/SQLite incompatibility should report the failing pin/feature/engine and smallest evidenced alternative. Preserve completed authorized work and failed checks in the local checkpoint; no silent stack change or claim that P1a passed.
