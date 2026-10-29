# Rivet research log

Research date: **30 October 2026, Asia/Singapore**. This is a substantive summary of evidence and reasoning, not a copy of entire upstream pages. Sources and snapshot metadata are retained in the [source register](source-register.md). Architectural recommendations remain proposals; no implementation was authorized or performed.

## Starting evidence and scope

Read AGENTS.md, GITHUB_DOCUMENTATION_GUIDE.md, root README.md and the project brief. Inspected the repository file list, clean starting status and local Git history: one initial documentation commit, `6fb21b00055e2beb82a02bda30a8f067693cbf7b`, on main with an existing origin/main tracking reference. No application, Cargo manifest, lockfile, test suite or workflow was present. No remote/publication operation was made during P0.5.

The first-README-only task restrictions were retired in the brief's introduction. All substantive product requirements and the historical local-development anecdote were retained; actual current Git history is recorded here instead of rewriting that historical passage. Created the requested planning set and added only root `/.research-local/` to existing ignore rules for optional raw scratch. No substantive finding is placed solely in ignored storage.

## Research sequence and conclusions

| Pass | Evidence considered | Findings, alternatives and effect on the proposal |
| --- | --- | --- |
| Runtime and storage | Tokio/clap/rusqlite/SQLx API/package docs; SQLite WAL, atomic commit and backup (S01–S05, S27–S28) | Reuse Rust async/CLI/SQL libraries. Proposed one bounded ledger worker and rusqlite rather than duplicate SQL stacks; SQLx remains an alternative. WAL permits reader concurrency but a single writer, with local-host assumptions. Database commits cannot atomically include shell/remote effects; explicit unknown/reconciliation states required. |
| SQLite version risk | WAL issue/fix and release history (S27) | Upstream documents WAL-reset fix in 3.51.3 and selected backports. Require actual linked engine evidence, not just wrapper version. No vulnerability scan or database exercise performed. |
| Frameworks | Rig, LangGraph, AutoGen and Microsoft Agent Framework (S17–S20) | Prefer narrow provider reuse and checkpoint concepts over importing overlapping schedulers/ledgers. AutoGen fetched README says archived, while older maintenance wording remains: reject as new foundation, record inconsistency. Rig main's split crates do not establish that published rig-core 0.45.0 exposes all new interfaces. |
| Providers and resources | llama.cpp server, Ollama API, Anthropic overview, llmfit (S23–S26) | Use tested capability profiles; compatibility does not imply universal tool calling/cancellation. Shared weights do not eliminate per-request cache or budgets. Local endpoint can route to cloud. Restrict llmfit integration to authorized fit data; benchmark/download/server commands are not implicitly permitted. |
| Security and cancellation | bubblewrap, Landlock, openat2, cgroup v2, OWASP, MCP security and Tokio process/shutdown (S02, S22, S29–S33, S49) | Broker rules need OS enforcement and explicit capability probing. bubblewrap is not a ready-made security policy. Descriptor-based operations help traversal/race handling. Dropped Child handles do not by default stop spawned processes. No prompt-only authorization or exactly-once shell claim. |
| Browser/native desktop | Playwright languages/library/Docker; AT-SPI, atspi/zbus and RemoteDesktop portal (S34–S39) | Official Rust Playwright binding absent from list; propose one supported-language process worker. Browser contexts are not host isolation; root can disable Chromium sandbox. Native semantic inspection and consented screen/input are separate capabilities and fixtures; no universal desktop claim. |
| Acquisition and updates | aqua security/config/lazy install, uv tools/environment/storage, TUF (S40–S42, S54–S55) | Prefer upstream-aware tools but qualify all paths/defaults. Disable unapproved lazy acquisition and arbitrary PATH fallback; constrain uv environments/cache/Python acquisition. TUF informs trust-root/expiry/rollback design, not proof of available upstream signatures. No custom general-purpose package manager or claimed compression savings. |
| Licenses and maintenance | Upstream manifests/license files, OSI/ASF/Mozilla/FSF texts (S03–S15, S21, S36–S48) | Record artifact/version-specific license evidence; MIT/Apache choices are common but not a whole-dependency clearance. MCP current license transition and LGPL native dependencies need explicit review. Compared permissive/file/strong/network copyleft without choosing a Rivet license. |
| Additional alternatives | Goose, gVisor, Secret Service, RustSec/cargo-deny (S50–S53) | Goose is a current Rust/MCP reference, not assumed to implement Rivet counts/recovery. gVisor is a stronger deferred boundary with compatibility burden. Latest Secret Service spec is a draft. Existing advisory/license tools are future development checks, not run here. |
| Product and complexity audit | Approved brief versus proposed components, modes, roadmap and inventory | Retained exact counts, no-default-model policy, one runtime/ledger, Portable and optional Unrivet boundaries. Reject redundant services and implicit distributed Mesh. Preserve missing per-ID inventory/version/license qualification as a real gap rather than inventing 109 entries. |

## Source discrepancies and access limitations

- rusqlite documentation returned 0.41.1, while moving master manifest returned 0.41.0. Keep both snapshots and reconcile at pinning.
- atspi package returned 0.30.0; main manifest returned 0.31.0 with a zbus 5.5 workspace requirement; separately fetched zbus latest was 6.1.0. Do not blindly combine newest crates.
- clap API prose included an older compiler-support statement, while fetched workspace manifest says Rust 1.85 and version 4.6.7. No Rivet MSRV inferred; selected release manifest and build evidence must govern.
- Rig repository root now documents a facade and split components, while rig-core has its own published version. The first guessed `docs.rs/rig-core/latest/rig/` URL returned 404; package metadata/repository were used instead.
- MCP main LICENSE reports a transition from MIT contributions to Apache-2.0 new code with separately licensed documentation. Selected rmcp release terms require inspection; avoid simplistic repository-license labels.
- cap-std package and published manifest used different repository identities (sunfishcode versus Bytecode Alliance). Guessed raw LICENSE path returned 404; published manifest supplied the actual license expression.
- AutoGen's explicit archived/no-longer-maintained warning coexists with older maintenance-mode wording. Current recommendation is conservative rejection; no claim of continued support.
- Goose's old block/goose URL redirected to aaif-goose/goose. Record canonical source without changing Rivet remotes.
- GNOME's initially guessed at-spi2-core generated-docs URL was inaccessible; docs.gtk.org/atspi2 supplied API/library/license metadata instead.
- llama.cpp's directory URL was restricted by the fetch tool; the explicit server README was accessible. No runtime or model downloaded.
- GNU GPL/AGPL/LGPL steward pages timed out. OSI's GPL/AGPL texts and bubblewrap's upstream LGPL text were read; Mozilla FAQ is explicitly explanatory rather than the MPL license itself.
- Guessed aqua `/reference/configuration/` and `/reference/configuration/environment-variables/` URLs were inaccessible. Search located `/reference/config/` and `/reference/lazy-install/`, exposing default lazy acquisition and PATH behavior that the initial generic security page did not establish.
- Explicit versioned docs.rs Cargo.toml fetches for clap/gix/rmcp/zbus were inaccessible in the web tool. Used available upstream manifests/license files; gix latest published manifest was subsequently discoverable. Revalidate selected immutable release evidence before acquisition. These failures are not evidence that the projects or versions do not exist.
- Latest documentation URLs can change and fetches may reflect independently cached snapshots. Versions are observed values, not future pins or continuous maintenance promises. No response times, throughput, hardware fit, footprint or support responsiveness was measured.

## Checkpoints and continuation

Initial source/architecture/agent drafts survived an intentional conversation interruption; continued from those files without discarding findings. No background application/build processes were started.

At the continuation checkpoint, ran authorized `codex-status`: **5-hour 94% remaining; weekly 11%**. This keeps the AGENTS.md sustained-work interval at 10 minutes; weekly allowance does not change that rule. Saved the source register and architecture/agent drafts, then added security/recovery, comparisons, roadmap, index and open-decision documents. User-provided usage outputs are check-ins, not permission to start another stage.

At the next shell checkpoint, `codex-status` reported **5-hour 88% remaining; weekly 10%**. All requested drafts were saved. Initial documentation checks passed for 10 planning documents, 57 local links/anchors, all 55 source IDs, structure/newlines/whitespace and scratch-ignore behavior. Continued to final diff/evidence review; no new implementation scope was started.

Final provider evidence review added observed upstream releases/licenses for optional llama.cpp (v0.8.0, MIT) and Ollama (v0.40.13, MIT). Neither runtime is selected or acquired; model licenses remain separate. No new implementation work was started.

## Documentation validation and final stage state

**P0.5 research/documentation complete.** Reviewed all requested topics and the staged changes. Created nine requested planning documents, changed the brief only in its obsolete first-task introduction, and added root research scratch to `.gitignore`: 11 files in the approved change set. No implementation, installation, model download, paid call, host change, sibling access, remote change, push or publication was performed.

Checks performed:

- Validated all 57 local links/anchors in 10 planning Markdown documents, the 55 source IDs, one title per document, heading order, balanced fences, final newlines and whitespace. Reviewed table structure and the Mermaid source. No rendered Markdown/Mermaid preview was available or performed; source checks are not a rendered preview.
- Checked all 98 distinct cited external URLs using public unauthenticated HTTP HEAD requests: all succeeded after redirects. This confirms reachability, not immutable content or future availability. Earlier source-body access failures and snapshot disagreements remain recorded above. Raw status metadata is retained locally in ignored `.research-local/link-check.json`; source URLs, observations, conclusions and check outcomes are preserved in tracked documents.
- Checked that scratch is ignored while planning/source-register/research-log, source files, Cargo.lock, Cargo.toml and safe environment examples remain trackable. No substantive research is hidden in ignored scratch.
- Compared the brief from `**Precedence:**` onward byte-for-byte with the initial commit: unchanged. Confirmed AGENTS.md, GITHUB_DOCUMENTATION_GUIDE.md and the root README are unchanged. Reviewed exact role counts, no-default-model policy, Portable/preset boundaries, extension independence and approval gates for consistency.
- Inspected staged diffs and ran `git diff --cached --check`; no whitespace errors. Scanned the 11 staged text files for common private-key, GitHub/API/cloud token, credential-URL and assigned-secret patterns; no matches. Also reviewed content for accidental credentials. Pattern checks do not guarantee detection of every secret.

No repository Markdown lint/build tooling exists, and none was installed. No application test/build, live inference, desktop/browser exercise, model benchmark or dependency/advisory scan was run. Those remain proposed future acceptance checks. D01–D15, artifact-specific dependency/licensing verification, complete historical inventory and platform/recovery measurements remain unresolved; the documentation does not claim otherwise.

The local P0.5 commit is the durable handoff checkpoint; obtain its hash from Git history. Stop after reporting this stage. No P1 or later stage is approved by these findings.

## P0.6 final planning — complete

Starting history: `759a327` on main and origin/main, with a clean tree. The user supplied a successful push of P0.5 after that stage ended. This corrects the index's stale publication context without granting an agent any future push permission. Read repository instructions, guide, root overview and all tracked planning/research documents; preserve P0.5 evidence and alternatives.

Initial shell allowance: five-hour 94%, weekly 99%; ten-minute checkpoint interval. At the next shell checkpoint, five-hour 88%, weekly 98%; saved the master architecture/stack/contracts/task sequence and public metadata summaries, continuing documentation only.

New research resolved published crate/license/MSRV/dependency edges using registry JSON (S56/S60), selected stable Rust 1.99.1 from the official announcement, and chose direct HTTP rather than another agent runtime. Selected official JavaScript Playwright/Node LTS by matching engine requirements; native atspi/ashpd must resolve zbus 5.x and retain async-io where upstream requires it. Landlock wrapper defaults to best-effort, so required rights must use strict compatibility policy.

SQLite review found the older rusqlite tag embeds 3.54.0 even though a latest docs constant showed a stale 0.38.2 / 3034001 snapshot. Neither proves sys 0.39.1's runtime engine. Select a non-yanked compatible sys 0.39.1 constraint and require actual linked engine >=3.54.1 at the first build; no unsafe system/vendor fallback. This is a narrow execution check, not an unresolved architecture study.

P0.6 source-access limitations: guessed Rust 1.92.0 date URL was invalid; official index established 1.99.1 instead. Versioned docs.rs manifests/sys constants remained inaccessible; registry dependency metadata plus rusqlite's versioned manifest/header were used. Guessed Landlock LICENSE path returned 404; registry license expression used. ashpd moving master differs from published metadata, so the published package governs candidate features/MSRV. Ubuntu 24.04 listing returned a 26.10 heading alongside 24.04 image filenames; only a proposed test target is inferred. Record these conflicts rather than invent validation.

One concrete environment blocker: Cargo reads ancestor configuration; CARGO_HOME within Rivet does not eliminate that behavior. The P1a handoff must require an existing contained execution route, or separately approved narrow automatic config-read access, before source edits/builds. No host settings were inspected to determine which route exists. No installs, source archives, binaries/models, builds, tests, paid calls, remote changes or pushes were performed.

P0.6 final-pass refinements: selected cap-std already in P1a for state/config boundaries, without exposing coding tools yet. Read-only status/list cannot initialize WAL/shared-memory as a side effect: selected quiescent locked immutable reads only, active/dirty state reports busy/recovery-required, and P2 introduces supervised live status (S66). Explicit URI flags are necessary; immutable is forbidden against a changing DB. Source-backed numeric limits are not claimed: selected cancellation/run/soak/retention numbers are engineering targets for owner stage adoption and later measurement.

All D01–D15 now have dispositions: D01–D12/D14–D15 selected technical recommendations plus named owner/verification gates; D13 intentionally remains owner-only. Published version constraints resolved the rusqlite/desktop ambiguity without overwriting P0.5 observations. Master and handoff are the current authoritative recommendation layer; historical analysis/alternatives remain linked and Git-preserved. P1a prompt includes no execution authorization until the owner sends it in a future turn.

Final compatibility correction: ashpd defaults to Tokio, while atspi-connection enables async-io. Selected ashpd defaults off/async-io and async-io 2.6.0 in the separate desktop worker, preventing an unqualified mixed-runtime zbus graph; the supervisor remains Tokio. Registry/API metadata was checked, not compiled. At the next shell checkpoint allowance was five-hour 83%, weekly 97%; all drafts and source evidence were saved.


### P0.6 validation and durable handoff

Created MASTER_IMPLEMENTATION_PLAN.md and P1_IMPLEMENTATION_HANDOFF.md; updated the index, D01–D15 dispositions, roadmap, four supporting-document authority notes and both evidence logs/registers. Eleven tracked planning files changed. Root README, brief, AGENTS.md, documentation guide and .gitignore remain unchanged. P0.5 sources/alternatives and actual publication history are preserved; the user push of 759a327 is documented without relaxing agent push restrictions.

Documentation checks passed for 12 planning Markdown files: 86 local links/anchors, one title, heading order, table shape, fence balance, newlines/whitespace, all 67 source IDs and all 15 decision rows. Public unauthenticated HEAD checks passed for all 147 distinct cited URLs after redirects; this establishes reachability, not immutable content or build compatibility. Raw registry/dependency/engine/link metadata remains locally under ignored .research-local; substantive conclusions, constraints, source URLs and limits are Git-tracked.

Reviewed the master/handoff, exact staged diff and requirements/security consistency, including approved role counts, unset provider/no default downloads, one ledger/runtime, artifact/replay ambiguity, root-contained paths, final capacities/presets and extension independence. Selected one desktop async-io runtime to avoid mixed zbus features. Read-only inspection policy explicitly prevents database/WAL/SHM creation and refuses live/dirty state; future tests must demonstrate that behavior. Repair/settings remain model-free planned deliverables under guarded P2/P6 tasks, outside bounded P1a.

Checked ignore rules preserve documentation/source/Cargo.lock while raw scratch stays ignored. Common private-key/token/credential-URL/assigned-secret pattern review found no matches, supplemented by content review; not an exhaustive secret-detection guarantee. git diff --cached --check passed. No Markdown/Mermaid rendered preview, compiler/Cargo invocation, application test/build, sandbox exercise, dependency resolution/advisory scan, model benchmark or live inference was performed. No software/source scaffold, installation, source archive/binary/model acquisition, outside-project filesystem inspection, sibling access, paid call, remote change, GitHub publication or push occurred.

P0.6 is the final general planning pass. Recommendations are implementation-ready defaults; runtime acceptance still requires evidence. Immediate gates are explicit P1a authorization, compliant existing build route or separately approved Cargo configuration-read exception, and bounded source-acquisition permission if needed. License/signing/release/support/inventory choices remain owner decisions at their assigned stages; no P1 permission is inferred. The local P0.6 commit is the durable checkpoint; get its hash from Git history. Report and STOP.
