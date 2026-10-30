# Post-P3 verification and publication gate

30 October 2026; baseline `7642495`, main, 15 commits ahead of the locally recorded origin/main. Owner authorization: [post-P3 brief](../POST_P3_RESEARCH_BRIEF.md). The only initially untracked file was that supplied brief. Prior P1–P3 evidence remains historical; this document records what was reproduced in this run.

## Reproduced checks

| Check | Actual result |
| --- | --- |
| `scripts/cargo-contained offline test --locked --offline --all-targets` | **77 passed, 0 failed, 8 ignored**, summed across test binaries. |
| `scripts/cargo-contained probe` | Passed: isolated ancestor/home/config/environment, read-only system and offline network. |
| `python3 scripts/verify-build-boundary.py` | Passed: actual Cargo build script/child inherits boundary. |
| Contained locked offline `build` | Passed. |
| Contained locked offline `check --all-targets` | Passed without warnings. |
| Contained `fmt --all -- --check` | Unavailable: no `fmt` subcommand. No installation. |
| Contained `clippy --locked --offline --all-targets -- -D warnings` | Unavailable: no `clippy` subcommand. No installation. |
| Actual CLI help/config/status/template | Eight invocations passed, including absent-provider inspection, Pair template and command-family help. |

Raw logs are ignored repository-local `.build/post-p3-tests.txt`, `post-p3-probe.txt`, `post-p3-build-boundary.txt`, `post-p3-build.txt`, `post-p3-check.txt`, `post-p3-fmt.txt`, `post-p3-clippy.txt` and `post-p3-cli.json`. Durable counts/results are recorded here; local logs are not a portable public test artifact.

## Coverage and classification

| Area | Reproduced evidence / remaining limit |
| --- | --- |
| Solo/files/permissions | Grant mutation/denial, unsafe links/mounts, stale revisions, protected paths, audit and reservation tests pass. Full synthetic read→patch→isolated check continuation is a native ignored fixture, not rerun. |
| Provider transports | Synthetic compatible/Ollama/Anthropic stream and usage/error/credential fixtures pass. No live inference, paid service or model capability verification. |
| Recovery | Child-process crash, durable intent/blob/restore activation fault and bounded storage/lock/reconciliation fixtures pass. Native old-worker containment/restore qualification not rerun. No real power-loss/reboot test. |
| Bounded autonomy | Seeded control/dispatch, deadline/clock/quota/resume and restart tests pass. Native detached-core/worker lifecycle not rerun. No endurance claim. |
| Pair/Crew/Fleet | Cardinalities, scheduler/lease/fence, provenance-bound review and consent/dedup/atomic acknowledgment tests pass. Native checked end-to-end hierarchy fixture not rerun. |
| Shared Model/Concurrent/Relay | Reservation/accounting, unknown holds, synthetic overlapping HTTP and phased relay tests pass. Physical fit and runtime load/unload remain owner declarations/manual observations. |
| Cookbook | Bounded qualified-shape import and ranking pass; llmfit is not executed. |
| Mesh/Adaptive | Deterministic peer ownership/conflict/resume and policy-bound transitions pass. No distributed cluster, live-model quality or unrestricted delegation qualification. |

Code/test review included CLI/config root restrictions; supervisor/broker grant and intent ordering; descriptor files; process/systemd bootstrap and strict Landlock; P2 repair/restore and lifecycle evidence; graph/model/Adaptive constraints and their regression tests. No reproduced defect required a P1–P3 code change. Passing this suite is bounded regression evidence, not an independent security audit or universal CAS guarantee against noncooperating host editors.

## Native authority and publication

Eight ignored tests require host user-systemd temporary units and `/proc`/`/sys/fs/cgroup` inspection/control outside the authorized project. Historical evidence records the prior run's explicit exception. This brief overrides only stage-stop approvals and does not explicitly grant a new host exception; those tests were not launched. Neither were `qualify-cgroups.py` or `qualify-worker.py`. Do not reuse an old planning assertion as current filesystem authorization.

**P1–P3 requalification is partial; publication gate is not satisfied. No push.** Historical native passes are retained, not represented as fresh acceptance. The configured origin is `https://github.com/zunobuilds/rivet.git` and branch is main; remote identity alone does not remove the gate. No live inference is necessary for the synthetic bounded contract, and no permission to call a model is requested or inferred.

Future requalification requires explicit narrow temporary-unit/cgroup/proc authority, then all eight native tests and owned worker/resource probes against unchanged source, with cleanup evidence. If acceptance fails, fix only approved P1–P3 defects and repeat affected checks. Any future push needs the then-current specific authorization; this run's unused conditional permission is not general publication permission.

## Documentation checks

Final command/link/structure/secret/diff results and commits are recorded in [the handoff](../POST_P3_HANDOFF.md). No browser-rendered Markdown preview, dependency vulnerability clearance, fresh dependency archive audit, package installation or release was performed. No AGENTS.md, license, source, lockfile or host configuration change was made.
