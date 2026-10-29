# P1a implementation evidence — 30 October 2026

Scope: complete P1a foundation only, authorized by the owner's implementation request and subsequent explicit build-runner exception. Baseline `3a4c59f`. No AGENTS.md/guide edits, sibling access, push, host modification, installation, live inference or later-stage implementation.

## Preflight and build containment

Installed tools observed: rustc 1.99.1 (`58cf8f9c3`, Fedora 1.99.1-2.fc44), Cargo 1.99.1 (`5f94df478`), GCC 16.2.1 (Red Hat 16.2.1-2), bubblewrap 0.12.0. This installed bubblewrap is used for the developer build route only; it does not replace the master's later P1b worker qualification or candidate version.

`scripts/cargo-contained` mounts the checkout at `/work`, selected system executable/header/library directories under read-only `/usr`, and the installed linker at its alternatives path. Host parent/home directories are absent. `/proc` belongs to a private PID namespace; Git metadata is masked. The environment is allowlisted, HOME/CODEX_HOME are not repurposed, and Cargo/cache/target/temp paths are repository-local. Symlinked build storage, project Cargo configuration and Cargo credential files are rejected. Offline commands have a private network namespace with no external route. Fetch alone shares network, exposes the system public CA bundle/resolver, and invokes Cargo fetch without build-script execution. Current Cargo.lock contains only public crates.io registry sources.

Initial failures and fixes: the environment probe needed to account for bubblewrap's synthetic PWD; the first compiler build could not resolve `/usr/bin/ld` through `/etc/alternatives/ld`, fixed with one read-only bind of installed `ld.bfd`. SQLite u64 conversion errors were corrected with checked signed conversion, without changing dependencies. The build-script probe initially assumed Cargo's working directory was always `/work`; it now accepts a contained descendant. A first narrow CA path was absent; the runner uses the existing public extracted CA bundle instead. A diagnostic-test refinement initially expected busy for an unknown-schema case; the test was corrected to expect schema/state failure while retaining the busy assertion for an active owner. Adding directory timestamps to snapshots exposed transient WAL creation/removal while rejecting a newer schema through a read-write connection. Existing state now passes an immutable schema check under the retained lock before any writable open; recovery remnants are refused for writers as well. Narrowing the system mounts also revealed that the synthetic sandbox root needed an explicit read-only remount; the final runner remounts it read-only while retaining the repository bind as writable. Both boundary probes pass with the final mounts. No tools or libraries were installed.

The final boundary probes test absent home/ancestor config and host credential paths, a synthetic inherited credential, read-only system files, isolated network interfaces and a failed connection to reserved address 192.0.2.1. `scripts/verify-build-boundary.py` repeats the boundary checks inside an actual Cargo build script and child, checking its output directory too. Generated fixture and binary stay in `.build/`. This is a Linux developer build boundary, not a malicious-host/kernel security claim or future process-tree/cgroup qualification.

## Implemented behavior

One Rust 2024 package/bin/lib; Rust minimum 1.99.1, Cargo.lock tracked, no license chosen. CLI: help/version, config check, status, explicit state init and session new/list. No default provider/model/endpoint, credential search, adapter, production dispatcher, download or inference readiness claim. Config schema v1 rejects unknown/malformed inputs with sanitized errors, including structured JSON diagnostics for JSON commands. Busy and recovery-required states have distinct diagnostic kinds. Config/state paths remain within the chosen project under this checkout, with descriptor-based access and no parent/home config discovery. `RIVET_SOURCE_ROOT`, supplied by the wrapper at compile time, also permits executing the binary from the original checkout outside its `/work` mount; this is not a relocation/distribution promise.

Schema v1 implements meta, sessions, runs, agents, tasks, operations, grants and ordered events. One retained exclusive cooperative lock and descriptor-root identity protect each ledger. Owner epochs fence transitions. A bounded 128-command dedicated writer thread commits state plus audit atomically, with WAL/FULL, foreign keys and 5-second SQLite busy bound. Standalone status/list acquire the existing lock without creating it, refuse active ownership/recovery remnants, and open only a quiescent immutable read-only database. File bytes, mtimes and directory contents are compared before/after CLI reads. Initial state permissions are 0700 directories/0600 files; links, unsafe roots/files and newer schemas fail closed. Clean shutdown checkpoints/closes before releasing ownership.

Versioned contracts cover runs, IDs/envelopes, provider events, descriptors/proposals, grants, receipts and ledger transitions. Solo admission requires exactly one Independent; all other named modes reject as unsupported. Registry rejects duplicate namespaced ID/version pairs. The broker denies production proposals; inert approval and dispatch exist only under cfg(test). Grants use opaque private references, exact proposal equality, epoch, expiry and one-use consumption. Persisted proposal/scope fingerprints are SHA-256 of canonical typed JSON, keeping raw arguments out of the ledger. A small local SHA-256 implementation avoids adding unapproved dependencies; published empty/abc/multi-block vectors are tested. Exact equality separately protects in-memory grant binding. No external cryptographic/security audit is claimed.

The actual supervisor's test-only dispatcher requires committed authorization and intent before the inert effect, then records the receipt/terminal state. Audit failure prevents dispatch. ProviderPort receives no executor/broker/ledger handle. Cancellation bypasses the ordinary event queue and cancels owned in-memory drivers; ledger cancellation has a separate token, drains pending replies and joins the writer. Receipt unknown/cancelled states cannot be promoted through the successful terminal transition. There is no shell/process enforcement, recovery/resume, hierarchy, real provider, MCP, browser or desktop implementation.

## Dependency evidence

All 113 resolved registry source archives match the SHA-256 checksums in Cargo.lock. `scripts/audit-dependencies.py` checks sources/checksums and declared compiler/license metadata using contained `cargo metadata` output. Highest declared transitive Rust minimum: 1.89.0; the entire active Linux graph compiles on 1.99.1. Features were inspected using `cargo tree -e features`. rusqlite transitively enables libsqlite3-sys defaults/min_sqlite_version_3_45_3 even with direct sys defaults disabled; bundled feature unification is active and no system-library fallback is used. No Tokio net/process features or later-stage application dependencies were added.

| Direct dependency | Exact resolved version | Selected features/defaults |
| --- | --- | --- |
| tokio | 1.54.1 | Defaults off; rt-multi-thread, macros, sync, time, signal |
| tokio-util | 0.7.20 | Defaults off; rt |
| clap | 4.6.7 | Ordinary defaults; derive |
| serde | 1.0.229 | Ordinary defaults; derive |
| serde_json | 1.0.154 | Ordinary defaults |
| toml | 1.1.8+spec-1.1.0 | Defaults off; std, serde, parse; requirement =1.1.8 |
| rusqlite | 0.41.1 | Defaults off; bundled, backup |
| libsqlite3-sys | 0.39.1 | Direct defaults off; bundled unified through rusqlite |
| cap-std | 4.0.3 | Ordinary defaults |
| thiserror | 2.0.21 | Ordinary defaults |
| uuid | 1.29.0 | Ordinary defaults; v4, serde |
| tempfile (tests) | 3.27.0 | Ordinary defaults; explicit contained parents |

Runtime `SELECT sqlite_version()` reports **3.54.1**, with a >=3.54.1 gate on every connection. The bundled header independently reports 3.54.1. SQLite's bundled source is public domain; the Rust wrapper/sys package metadata declares MIT. Declared graph license expressions comprise MIT; MIT OR Apache-2.0 (including reversed/slash spellings); Apache-2.0 WITH LLVM-exception (alone or as an option); Unlicense OR MIT; MIT OR Apache-2.0 OR LGPL-2.1-or-later; and (MIT OR Apache-2.0) AND Unicode-3.0. Alternative license expressions are preserved, not silently selected as Rivet's license. No package is missing a declared license in this resolution. This metadata/source-build review is not a distribution license/advisory clearance; no source caches are committed and no release is authorized. No advisory database was fetched or full vulnerability audit claimed.

## Commands and results

- `scripts/cargo-contained probe`: passed.
- `python3 scripts/verify-build-boundary.py`: passed, actual Cargo build-script/child isolation.
- `scripts/cargo-contained fetch` then `fetch --locked`: passed; source acquisition only.
- `scripts/cargo-contained offline build --locked --offline`: passed.
- `scripts/cargo-contained offline check --locked --offline --all-targets`: passed without warnings after corrections.
- `scripts/cargo-contained offline test --locked --offline --all-targets`: passed, 6 unit and 5 integration tests at this checkpoint.
- `scripts/cargo-contained offline fmt --all -- --check`: failed, `no such command: fmt`; component absent, no installation attempted.
- `scripts/cargo-contained offline clippy --locked --offline --all-targets -- -D warnings`: failed, `no such command: clippy`; component absent, no installation attempted. Compiler checks are not a substitute for Clippy.
- Contained `cargo tree --locked --offline -e features` and `cargo metadata --locked --offline --format-version 1`: passed; generated outputs ignored under `.build/`.
- `python3 scripts/audit-dependencies.py`: passed, 113 checksum/source/license/compiler metadata checks.
- Actual CLI `status --json` and `config check --json` from repository root: passed with missing state/provider and execution unavailable. Integration tests also run real help/version/state/session commands in contained fixtures and verify JSON plus before/after filesystem snapshots.

Tests cover config errors/no parent discovery, usable absent-provider administration, configured-unverified status, no read-time state creation/mutation, unsafe/link paths and permissions, retained locks, recovery remnants, newer schema, injected atomic rollback, stale owner/grants and mutated scope, duplicate registry, Solo rules, provider authority, intent ordering/critical-failure non-dispatch, queue-independent cancellation and engine version. All fixture temp parents are explicit `.build/tmp`. No tests construct a real network provider. Power-loss recovery, hostile same-user mutation, installed worker sandbox behavior and broader Linux support remain unqualified. Formatting/Clippy remain outstanding environmental checks.

Final review: repository-local Markdown file/anchor links checked; inherited external links were not re-fetched and no Markdown rendering tool was installed. Complete source/manifest/test/runner/documentation diff reviewed, Cargo.lock checked against all acquired source archives, whitespace checked with `git diff --check`. No actual keys or tokens were added; the only credential-like strings are explicitly synthetic fixture sentinels. AGENTS.md and the documentation guide remain unchanged. Commit is local on main; P1a stops here, with missing formatter/Clippy qualification explicitly outstanding.
