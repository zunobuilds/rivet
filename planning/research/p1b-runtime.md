# P1b runtime qualification — 30 October 2026

Continuation of [extended-run evidence](extended-run.md). The owner explicitly authorized read-only `/sys/fs/cgroup`, relevant `/proc`/user-systemd inspection, temporary unprivileged user units and writes only inside explicitly delegated Rivet subtrees. Prior commits `090f47c`/`25fce90` are preserved. No installations, persistent host configuration changes, credentials, sibling access, pushes or remote changes.

## Verified delegation and isolation route

User systemd 259.10 is running. cgroup v2 is mounted and user-manager delegation includes CPU, memory and PID controllers. `scripts/qualify-cgroups.py` creates a UUID-named `rivet-cgroup-*.service` with delegation; it moves only its own probe into an owned supervisor leaf, configures only an owned worker leaf, verifies limits and cleans both leaves and the transient unit.

Actual positive probe: cpu.max `20000 100000`; memory.max `67108864`; memory.swap.max `0`; pids.max `4`. Observed 16 CPU throttles / 323130 microseconds CPU usage over the 1.5-second workload, fork denial after 3 children, one OOM kill, and cgroup.kill emptied a three-process detached tree in 1.25 ms. Unit LoadState was not-found after cleanup. These are this host's probe results, not general performance guarantees. The first memory probe timed out with swap use; explicitly disabling swap fixed the intended aggregate bound. Its failed transient unit was also stopped/collected.

`landlock =0.4.7` and `tracing =0.1.44` are now source-fetched and used. Tokio adds process/io-util features for bounded process I/O. All original pins remain. The graph has 120 checksum-verified public registry archives (no git/private sources); all declared compiler minima remain <=1.89.0 and the active graph builds on Rust 1.99.1. Source/license metadata audit passes; no distribution license or vulnerability-audit clearance is claimed. SQLite remains bundled 3.54.1.

The installed bubblewrap is 0.12.0, not the master's 0.13.1 candidate. The owner authorized existing containment tools; this specific older installation is being qualified against actual required operations, not silently represented as the selected version. Explicit --unshare-user is necessary alongside --unshare-all for --disable-userns on this installation. No setuid fallback, --unshare-user-try or --not-a-security-boundary is used. The version-specific profile remains limited to measured tests below; no cross-platform/support/advisory claim.

## Implemented process foundation

`src/tools/process.rs` defines a typed, bounded isolated-check request and worker bootstrap. Inputs are an explicit immutable projection, not an ambient mount of the project, state, Git metadata, home or credentials. Projection data is prepared under a private repository-local `.build/runtime/<operation>` directory with descriptor-relative no-link/no-mount path handling. No automatic runtime/model/dependency acquisition occurs. Only `/usr/bin/...` commands are admitted, with exact argv/projection/profile in the proposal digest and user-reviewed resources. There is currently a trusted Rust host API, no arbitrary CLI/model approval bypass.

The systemd backend creates only `rivet-worker-<operation-UUID>.service` with CPUQuota 100% / 100ms period, MemoryMax 512M, MemorySwapMax 0, TasksMax 32, KillMode control-group, bounded runtime and 5-second stop grace, and NoNewPrivileges. A single-threaded trusted service entry verifies actual kernel limit values in its own UUID-matched cgroup before executing bubblewrap. It does not change other units or global controller settings.

Bubblewrap exposes selected installed executable/library/header directories read-only, a read-only input projection, private PID/network/user/IPC/UTS/cgroup namespaces and no home, credential directory or user bus. Nested user namespaces are disabled and capabilities dropped. Scratch is a 64 MiB private RAM filesystem under the worker's `/work` view, charged to the worker cgroup; it is discarded, not written to host `/tmp`. Persistent projection/evidence/cache data stays under Rivet. Commands cannot modify the actual project through this check profile. Outputs from a real project's complex build may need separately qualified assets/profiles; no writable whole-project fallback is added.

Before untrusted exec, the bootstrap requires NoNewPrivs, no inherited non-stdio descriptors, and fully enforced Landlock ABI-v6-required filesystem, TCP and signal/abstract-socket scope restrictions using HardRequirement. System/input/proc views are read-only, only scratch has general write rights, and TCP has no allow rules. Unsupported enforcement prevents the requested program from executing. This is applied anew for each worker rather than trusting a cached readiness flag.

The first attempted input handoff through the parent's `/proc/<pid>/fd` was denied across the systemd boundary. The final handoff opens the projection inside the service with openat2 restrictions, checks its device/inode against the supervisor-retained identity, and passes its own descriptor to bubblewrap's ro-bind-fd. The bootstrap verifies that no descriptor survives into untrusted exec. No ptrace/LSM/host policy was weakened.

The supervisor records broker admission, authorization, intent, dispatch and receipt; output retained in the ledger is fingerprints/counts, not raw process text. stdout/stderr capture is bounded to 32 KiB each, with explicit truncation and worker cancellation on overflow. Cancellation and broker revocation are separate control tokens and bypass ordinary queues. Ledger failure also cancels affected running work. Grant expiry can only shorten the requested run bound. Permission epochs are now persisted separately from owner-fencing epochs; revocation does not require pretending ownership changed.

Owned tasks terminate only their UUID-named unit, wait for cleanup and remove their private projection. Drop requests cleanup as well. After supervisor crash, RuntimeMax bounds residual lifetime; crash-time reconciliation is still P2 work and is not claimed implemented. Cancellation cannot undo completed effects. No kernel-I/O interruption or universal noncooperating-editor CAS guarantee is claimed.

## Actual checks at this checkpoint

- Contained offline build/check: passed without warnings after wiring the implemented process route.
- Contained `cargo test --locked --offline --all-targets`: 19 tests passed; 1 native runtime test explicitly ignored in the developer namespace because it has no user bus/cgroup exposure.
- Native execution of that already-offline-built runtime test binary with `--ignored --test-threads=1`: passed. It tests supervised normal output, output flooding/bounds, broker revocation and projection/unit cleanup against real user-systemd services. This is a separate real check, not an ignored-test pass claim.
- `python3 scripts/qualify-cgroups.py`: actual CPU/PID/memory/swap enforcement, detached descendants and cleanup passed.
- `python3 scripts/qualify-worker.py`: actual no-network/filesystem/fully-enforced-Landlock private-view probe passed; unit collected afterward.
- Dependency source/checksum/license/compiler metadata audit: passed for 120 registry archives. Rustfmt/Clippy attempted again; components absent, no installation.

P1b is still partial: full file-race/lease work, permission-controlled Git and the P1b.4 patch/check demonstration remain. P1c–P3c remain unstarted at this checkpoint. The former missing-delegation authority blocker is resolved for the explicit user-systemd route; the older repository-only `sandbox check` still reports the separate directory-based route honestly, and is not a global worker-readiness certificate. Continue the approved stage order; do not infer P1b acceptance from this runtime checkpoint alone.
