# Extended implementation run — 30 October 2026

Owner authorized P1a cleanup and P1b through P3c in dependency order, with stage-approval/stopping gates waived for this run only. All other repository/permission boundaries remain. Baseline: `176bbb77bc86b3477d569962f148ba610b2edaf4`. No push authorized.

## P1a cleanup checkpoint

Review found and fixed three state/authority defects:

- Orphan WAL/SHM/journal remnants with a missing main database now reject initialization as recovery-required; no empty replacement database can hide recovery evidence.
- Once a command is accepted into the writer queue, cancellation waits for its definitive commit/rollback reply. Admission remains independently cancellable; queued commands still receive a drained interruption reply. A synchronized queue/oneshot test proves cancellation cannot overwrite an accepted command's outcome.
- Existing metadata must have exactly one row, a valid project UUID, nonnegative owner epoch and an event sequence consistent with the ledger. Inconsistent state fails before a writable open.

Checks: contained boundary probe passed. `cargo test --locked --offline --all-targets` passed (7 unit, 6 integration). Rustfmt and Clippy were attempted through the contained route and remain unavailable (`no such command`); no installation. Dependency graph unchanged. SQLite remains bundled 3.54.1. Regression tests compare file/directory snapshots around rejected initialization/inspection.

The P1a evidence remains the historical implementation record. This file records subsequent changes and the current handoff; planning/README.md is updated for current status. No recovery feature or later-stage acceptance is implied by these defensive checks.

## P1b partial checkpoint and blocking prerequisite

**P1b has not passed stage acceptance.** The independent file foundation is implemented; terminal/Git/coding-agent execution is unavailable. No P1c–P3c work started because the master's dependency rows require accepted P1b/P1/P2 in order. This run stops on the environment/authority gate below, not on usage exhaustion or an initial ordinary build failure.

### Implemented file foundation

- `src/tools/files.rs`: descriptor-root text read/search/create/write/literal patch/rename/delete, 64 KiB input/output/diff limits, 1024-entry/32-depth search limits and explicit subdirectory searches. A literal patch requires exactly one matching anchor. Expected revisions bind content digest plus device/inode; writes stage, sync, revalidate, atomically replace and sync the parent. Create/rename use Linux `RENAME_NOREPLACE`; existing destinations are never silently overwritten. Existing permission bits are preserved without special setuid/setgid bits. Staging files are cleaned on failure; post-mutation I/O uncertainty is conservatively outcome-unknown.
- Safe descriptor-relative `openat2` uses BENEATH/NO_SYMLINKS/NO_XDEV, with no fallback on unsupported kernels. Tests include a same-filesystem bind mount that must still be denied, symlinked parents/leaves, hard links and FIFO special files. State, Git metadata, build/cache paths, Rivet config and credential-like paths are protected. Custom state roots are explicitly supplied to the workspace capability.
- File actions are registered as `rivet:files` v1 in the shared registry. A trusted host caller supplies the reviewed resources to the broker; ProviderPort has no broker/dispatcher capability. Grants bind exact typed proposal, actor/operation/attempt, workspace device/inode plus logical resource paths, owner/permission epoch and expiry. The supervisor checks the run's actual ledger project and workspace identities. Scope mutations and cross-workspace reuse cannot dispatch.
- `Supervisor::execute_file` consumes the exact approval before any file content read/write and records proposal/denial or authorization, intent and dispatch before the effect. Receipt records before/after digest references and a redacted diff fingerprint; content/diffs return only to the trusted caller. No fake artifact reference is invented: persistent blobs/diff artifacts remain P2 work. Critical ledger failure prevents dispatch. Blocking file work belongs to the supervisor TaskTracker, with cooperative cancellation checks before namespace mutations. This is a Rust host API used by synthetic tests, not a CLI approval workflow or provider coding loop.

Promoted **rustix 1.1.6**, already in the existing lockfile and source cache, to a direct exact dependency with defaults off and std/fs features. This narrow supporting-stack addition supplies safe openat2, fstatfs and no-clobber rename APIs that cap-std alone does not expose. It avoids unsafe application syscalls and racy check-then-clobber rename for new destinations. License expression: Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT; declared Rust minimum 1.65. Existing master pins remain unchanged; Cargo.lock changes only the root dependency edge. Still 113 checksum-verified crates.io source archives; no new package version or binary acquisition.

Initial P1b check compilation caught an over-broad edit placing initialization code in the queue-test fixture; corrected before testing. The nested bind-mount test initially failed because bubblewrap requires `/tmp` in its parent namespace. The contained developer wrapper now binds **repository-local `.build/tmp`** at `/tmp` as well as preserving TMPDIR there. The containment probe verifies both paths' device/inode identity, so no host temporary directory or outside files are exposed. The actual nested mount test and Cargo build-script/child isolation checks now pass. No host mount/configuration was changed: namespace mounts are private and temporary.

### Worker authority gate

`src/sandbox.rs` and `rivet sandbox check [--control-dir PATH] [--json]` implement read-only preflight. There is no default host discovery. An explicit control path must remain inside the project and is examined through a retained descriptor. Kernel fstatfs must report CGROUP2_SUPER_MAGIC; ordinary directories containing files named cgroup.controllers cannot stand in for real enforcement. Presence of cpu/memory/pids controllers is only a candidate, never proof of writable delegation or qualified process enforcement. Until those checks and worker implementation are completed, readiness remains false.

Actual command:

```bash
scripts/cargo-contained offline run --locked --offline -- sandbox check --json
```

Result: `ready=false`, `controls=not-configured`, exit **3**. Repository-only filename search (including ignored local directories, without following symlinks) found no `cgroup.controllers` file. The existing developer namespace exposes no cgroup controls; it is not a qualified runtime-worker profile. **Host cgroup availability was not inspected and is unknown.** No controller files, host credentials, unrelated repositories or Unrivet were accessed.

Master P1b.2 requires delegated cgroup limits, strict Landlock, no-network namespace execution and qualified process-tree cancellation. Reusing the developer runner alone or substituting per-process limits does not prove aggregate process-tree enforcement. No such fallback was enabled. Installing tools, changing system delegation/mounts, or discovering/writing host controller paths would exceed the current permission boundary. No existing compliant delegated route was supplied or found inside the authorized repository. Landlock/tracing/provider/MCP dependencies were not fetched merely to create later scaffolding while this prerequisite remained blocked.

### Verification at this checkpoint

- Contained `cargo check --locked --offline --all-targets`: passed without warnings.
- Contained `cargo test --locked --offline --all-targets`: **19 passed**: 8 unit, 7 administrative integration, 4 supervised file integration. Tests exercise actual file effects, grant/identity rejection, pre-mutation human-edit conflicts, output bounds, no-clobber rename, critical audit failure, cancellation before dispatch, literal patch ambiguity, protected/link/FIFO paths, and a nested same-filesystem bind mount.
- `scripts/cargo-contained probe` and `python3 scripts/verify-build-boundary.py`: passed after the repository-local /tmp alias change.
- Contained `cargo metadata`, feature tree and `python3 scripts/audit-dependencies.py`: passed; all 113 source archive checksums still match Cargo.lock. SQLite remains **3.54.1**.
- Contained Rustfmt/Clippy commands retried: unavailable (`no such command: fmt` / `clippy`). No installation or substitute pass claimed.
- CLI worker preflight: expected unsupported exit 3 without file creation; synthetic controller-lookalike directory rejected. This is a passing refusal test, not a successfully qualified worker.

### Limits and exact resumption handoff

P1a cleanup is complete apart from unavailable formatter/Clippy qualification. P1b file primitives and supervised library routing are tested, but full P1b.1 concurrent-edit/lease qualification and P1b.2–4 acceptance are not complete. File serialization is cooperative per workspace object. A noncooperating editor can still race the narrow interval between final precondition validation and replacement/delete/rename; no universal atomic path/content compare-and-swap or crash-recovery guarantee is claimed. Kernel I/O cannot be forcibly interrupted by the in-memory cancellation token. There is no production terminal/Git adapter, provider, coding CLI, process-tree enforcement, crash reconciliation, backup/restore, detached autonomy or P3 execution mode. These are required remaining work, not hidden fallbacks. P2b's actual 72-hour qualification has not started and must never be replaced by a claim based on a short test.

**Required next input/environment:** an owner-supplied existing delegated cgroup-v2 subtree with cpu, memory and pids controls, and narrowly scoped authority for Rivet's per-worker controller/lifecycle operations. Prefer an owner-provided repository-contained controller view/handle; otherwise the exact existing outside path and allowed reads/writes require separate authorization. No broad host discovery, installation, system reconfiguration or uncontrolled privilege is requested. Supplying ordinary fixture files is insufficient.

Then resume at **P1b.2**, implement/qualify strict Landlock and namespace/resource limits, executable/argv/cwd/environment identities, bounded output, process-tree cancellation/reaping and revocation fixtures. Qualify the installed bubblewrap 0.12.0 against the selected profile or use an owner-provided already-installed qualified version; the master runtime candidate remains 0.13.1, not silently declared satisfied. Complete outstanding file race/lease checks and P1b.3 local Git helper/hook protections, then P1b.4's reviewed patch plus configured check demonstration. Only after P1b acceptance proceed to P1c, P1d, P2a/P2b (including the real soak), and P3a–c with their original contracts/counts and gates. Read the latest owner instructions plus this checkpoint; this document records authorization history, it cannot grant new outside access.

Final checkpoint review: revised README/planning-index/handoff local files and anchors passed; source/manifest/test/runner diffs and whitespace checked; staged secret-pattern/artifact review passed, with no AGENTS.md/guide changes. Existing external reference links were not re-fetched and no rendering tool was installed. Latest usage check before commit: five-hour 85% remaining; usage thresholds did not cause the stop. Stop reason is the P1b.2 authority/environment gate. All completed work is saved in ordinary local commits; no push or remote change.


## Current continuation pointer

The owner subsequently authorized the narrow user-systemd/cgroup investigation and temporary-unit writes. The old delegation blocker above is resolved; [P1b runtime qualification](p1b-runtime.md) records actual enforcement, namespace/Landlock/process/Git fixtures and the bounded P1b acceptance checkpoint. P1c is next in the approved sequence. Historical claims above are preserved as checkpoint evidence, not the current blocker.
