# Technology comparisons and feasibility

**P0.6 authority note:** The [master implementation plan](MASTER_IMPLEMENTATION_PLAN.md) selects the current stack, defaults and contracts; [decision dispositions](open-decisions.md) distinguish owner approvals from technical recommendations. P0.5 comparisons and proposal wording below are retained as research provenance, not competing current defaults. No implementation is authorized.

**P0.5 research snapshot: 30 October 2026.** No dependencies are selected, downloaded, built or benchmarked. Recommendations are proposals. The [source register](research/source-register.md) owns upstream URLs, observed versions, license evidence, access date and maintenance limitations; this document owns comparisons. [Open decisions](open-decisions.md) records approval/qualification gates.

## Selection principles

Prefer existing maintained libraries and upstream-supported protocols, but do not import a second permissions/scheduling/persistence system merely to gain one adapter. Evaluate feature-level fit, transitive licenses, compiler/platform requirements, cancellation and offline behavior. Published releases are evidence of availability, not proof of security support or compatibility. Moving-main observations and package versions are deliberately distinguished in the register.

Maintain a small explicit dependency set with only needed features. Preserve `Cargo.lock` for reproducible application dependency resolution once a Cargo project is separately approved. Determine the minimum supported Rust version (MSRV) from pinned dependencies and CI evidence; upstream pages currently disagree about some compiler-support details, so no Rivet MSRV is proposed as fact.

## Rust application building blocks

| Need | Candidates and observed versions | Comparison and recommendation | Evidence |
| --- | --- | --- | --- |
| Async execution | Tokio 1.54.1 | Proposed default: reusable tasks/channels/process/signal support; keep blocking ledger work off executor threads. Runtime cancellation alone is not child-process cancellation. | [S01](https://docs.rs/tokio/latest/tokio/), [S49](https://docs.rs/tokio/latest/tokio/process/index.html) |
| CLI | clap 4.6.7 | Prefer generated parsing/help over a custom parser. Terminal chat rendering is a separate concern; avoid exposing speculative command syntax. | [S03](https://docs.rs/clap/latest/clap/) |
| Terminal | Crossterm 0.29.0; Ratatui 0.30.2 | Start with chat output/input appropriate to the task; Ratatui is optional if structured rendering justifies it. A terminal UI does not require a file manager or dashboard. Test restoration, resizing and escape sanitization. | [S10](https://docs.rs/crate/ratatui/latest), [S11](https://docs.rs/crate/crossterm/latest) |
| Serialization | Serde 1.0.229 | Prefer typed envelopes plus explicit schema/version checks; serialization alone provides no semantic validation or compatibility policy. | [S08](https://docs.rs/crate/serde/latest) |
| HTTP/TLS | reqwest 0.13.5; rustls 0.23.46 | Proposed transport stack; select TLS backend/features explicitly and audit its crypto dependencies. Define redirect, proxy, certificate and timeout policy. Do not disable certificate validation for convenience. | [S06](https://docs.rs/crate/reqwest/latest), [S07](https://docs.rs/crate/rustls/latest) |
| SQLite access | rusqlite 0.41.1 docs; SQLx 0.9.0 | Prefer rusqlite plus one bounded ledger worker for a local SQLite-only scope; SQLx is a viable async/migration/query-tooling alternative. Do not add both. Reconcile the rusqlite manifest discrepancy before pinning. | [S04](https://docs.rs/rusqlite/latest/rusqlite/), [S05](https://docs.rs/crate/sqlx/latest) |
| Diagnostics | tracing 0.1.44 | Reuse spans/structured diagnostics; redact at source. Required audit transitions remain in the ledger. A second logging service is unnecessary. | [S09](https://docs.rs/crate/tracing/latest) |
| Credentials | keyring 4.2.0 | Reuse platform backends; qualify unlocked/locked/headless service behavior. Explicit ephemeral secrets can be an alternative; no silent plaintext fallback. | [S12](https://docs.rs/crate/keyring/latest), [S50](https://specifications.freedesktop.org/secret-service/latest/) |
| Filesystem capability | cap-std 4.0.3; openat2 | Prefer existing descriptor/capability abstractions and kernel resolution controls over custom string-path validation. Still need external-process containment. | [S13](https://docs.rs/crate/cap-std/latest), [S31](https://man7.org/linux/man-pages/man2/openat2.2.html) |
| MCP | rmcp 3.8.0 | Prefer official Rust SDK with a pinned negotiated protocol and minimal client transports. Tool roots/annotations do not enforce host permissions. Review current mixed-license transition at the selected release. | [S21](https://github.com/modelcontextprotocol/rust-sdk) |
| Git | Existing Git CLI; git2 0.21.0; gix 0.90.0 | Propose CLI first for familiar behavior and inspectable receipts; sandbox hooks/helpers. git2 adds native libgit2 dependencies; gix avoids that binding but needs operation-by-operation parity tests. Do not maintain three Git engines. | [S16](https://git-scm.com/docs/githooks), [S14](https://docs.rs/crate/git2/latest), [S15](https://docs.rs/crate/gix/latest) |

Preliminary licenses: Tokio/tracing/rusqlite/Crossterm/Ratatui are MIT in inspected evidence; clap/Serde/reqwest/SQLx/keyring/git2/gix offer MIT or Apache-2.0; rustls additionally offers ISC; cap-std includes an LLVM-exception option. These observations apply to those packages, not every dependency. SQLite is public-domain upstream code. rmcp needs contribution/release-specific license review. Exact evidence and gaps are retained in the register rather than replaced by a blanket “all permissive” claim.

## Framework and existing-agent alternatives

| Option | FOSS/license and maintenance snapshot | Benefits | Fit and proposed disposition |
| --- | --- | --- | --- |
| Narrow Rust core using existing libraries | Rivet license undecided; no implementation | Direct ownership of approved mode invariants, broker and unified ledger | Recommended architecture direction, but costs explicit state-machine/recovery engineering; not justification to rewrite HTTP/MCP/SQL libraries. |
| Rig provider/message layer | MIT root manifest; rig-core 0.45.0 published; moving main split into more crates | Reusable provider abstractions, streaming and tools; current main also documents memory/effect/run components | Evaluate a pinned provider-only integration versus direct reqwest adapters. Do not adopt another autonomous loop that can bypass the broker. Main and package are not one stable API snapshot. [S17](https://github.com/0xPlaygrounds/rig) |
| LangGraph | MIT; 1.2.18 observed release; Python and JS interfaces | Existing graph persistence and human-intervention concepts | Strong design reference/alternative if language constraints change; embedding creates another runtime and state owner. Do not copy its advertised durability into an exactly-once claim. [S18](https://github.com/langchain-ai/langgraph) |
| Microsoft Agent Framework | MIT; observed dotnet-1.26.0 release, Python/.NET project | Existing multi-agent orchestration and provider integration | Maintained upstream alternative to assess, but not Rust-native; extra runtime and overlapping scheduler make it a weak default for this product boundary. [S20](https://github.com/microsoft/agent-framework) |
| Goose | Apache-2.0; v1.57.0 observed release; Rust CLI/API and MCP extension model | Existing coding-agent architecture to study; possible reuse of appropriately scoped components after license/API review | Evaluate reference interfaces or libraries rather than silently turning Rivet into a distribution of another runtime. No evidence yet that its ownership/recovery contracts meet Rivet's requirements. [S51](https://github.com/aaif-goose/goose) |
| AutoGen | README explicitly says archived/no active maintenance; older maintenance text remains | Historical multi-agent patterns | Reject as a new foundation on current evidence. Upstream directs users to Agent Framework. No need to pin an abandoned candidate merely to complete a comparison table. [S19](https://github.com/microsoft/autogen) |

No framework has been run or audited here. Choosing Rust and Rivet ownership does not make an entirely new agent framework necessary: compose reusable providers, protocol libraries and storage around the smallest application-specific controller.

## Provider interfaces and model feasibility

| Surface | Supported upstream evidence | Tradeoff and proposed adapter policy |
| --- | --- | --- |
| Existing llama.cpp server | v0.8.0 release observed; MIT upstream; slots/batching and compatibility APIs, model/template-dependent tool calling | Optional local endpoint; qualify schemas, context allocation, usage and cancellation. Pin endpoint profile independently of model weights/license. No runtime or model auto-install. [S23](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md) |
| Ollama native/compatibility API | v0.40.13 release observed; MIT upstream; documented local and cloud surfaces | Easier provider setup where already installed; local daemon address alone does not establish offline processing. Block automatic cloud routing/download operations outside policy. [S24](https://docs.ollama.com/api/introduction) |
| Hosted Messages API | Anthropic documents versioned headers/authentication and streaming facilities | Direct HTTP or qualified library adapter; provider terms, credentials, quotas and project data transfer are separate from FOSS client licensing. No API call was made. [S25](https://platform.claude.com/docs/en/api/overview) |
| Compatibility endpoint | llama.cpp and Ollama document compatibility profiles | Useful common baseline, not universal feature parity. Capability-test tool IDs, streamed arguments, stop reasons, token usage and context failures. Unsupported features should fail explicitly. |
| llmfit fit/recommendation data | MIT workspace 1.1.17 main snapshot; JSON commands visible | Reuse estimates, then calculate hierarchy-wide resources. Guard against invoking download, serve or benchmark functionality implicitly. Estimates and observed measurements must remain distinct. [S26](https://github.com/AlexsJones/llmfit) |

Provider conformance should begin with deterministic fake transports and recorded synthetic events: incomplete streams, duplicate call IDs, malformed arguments, retries, quota errors and usage uncertainty. Optional later live checks require authorized credentials/data/cost. No cloud fallback or inference-ready status can be inferred from a library constructor succeeding.

## Sandboxing and desktop/browser feasibility

| Choice | Evidence/license/version snapshot | Feasibility and tradeoff |
| --- | --- | --- |
| bubblewrap + broker policy | 0.13.1; COPYING LGPL 2.1 text, mixed repository licenses | Lightweight namespace building block; policy, kernel support, syscall interactions and D-Bus exposure need qualification. [S29](https://github.com/containers/bubblewrap) |
| Landlock + descriptor APIs | Kernel ABI-dependent; cap-std license choices above | Defense in depth for built-ins/children; cannot replace process supervision or destination-aware network control. [S30](https://docs.kernel.org/userspace-api/landlock.html) |
| gVisor | Apache-2.0 repository; living architecture docs, no release selected | Stronger userspace application-kernel boundary to investigate for high-risk workloads; adds runtime and syscall/desktop compatibility burden. Deferred alternative, no unmeasured performance promises. [S52](https://gvisor.dev/docs/architecture_guide/intro/) |
| Official Playwright process adapter | Playwright 1.64.1 Apache-2.0; official JS/TS, Python, Java, .NET | Prefer a bounded stdio worker using official JS/TS or Python APIs over an unqualified Rust binding. Choose one runtime at P4; JS/TS offers upstream test runner, Python may reuse uv packaging. Chromium and OS dependencies remain separate. [S34](https://playwright.dev/docs/languages) |
| Community Rust Playwright binding or direct browser protocol | Not version/license-qualified here | Deferred: potential protocol drift and reduced upstream support; direct protocol would duplicate Playwright capabilities. Not needed to keep the supervisor Rust. |
| AT-SPI through Rust atspi | Crate 0.30.0; Apache-2.0 OR MIT; main 0.31.0 differs | Promising semantic Linux inspection without assuming libatspi linkage. Match zbus/runtime features; test application/toolkit gaps and access control. [S37](https://docs.rs/crate/atspi/latest) |
| libatspi and portal APIs | AT-SPI docs API 2.0/library 2.52 LGPL-2.1-or-later; RemoteDesktop v2 | Established native interfaces; C binding is an alternative with linking obligations. Portals supply consented input/screen sessions, not an accessibility tree. [S36](https://docs.gtk.org/atspi2/), [S39](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html) |

Recommendation: qualify GNOME and KDE/Wayland behavior explicitly, with X11 coverage recorded separately. Do not advertise all-Linux desktop support based on a successful browser test or one accessible application. Which distributions, compositors and toolkits are supported is unresolved (D09).

## Tool acquisition, presets and updates

Preserve independent Minimal/Recommended/Full/Custom presets and Portable recommended/System explicit opt-in/no Hybrid. Full means a qualified broader set, not all historical inventory entries. Model/provider choice remains independent, and no preset installs inference runtimes or weights by default.

Propose a tool receipt per selected artifact: canonical upstream, release and platform/ABI, license/notices, archive/content digest, available trusted signature/provenance, acquisition authorization, smoke-test outcome, dependencies and transformation lineage. Reuse immutable matching assets across Rivet/Unrivet and projects; a shared asset identity does not share project permissions.

**aqua requires policy configuration, not blind adoption.** Its documented lazy installation is enabled by default, and its proxy can resolve missing tools through PATH. Propose disabling lazy installation, pinning an explicit registry/configuration, using approved immutable executable paths and `AQUA_ROOT_DIR` within Portable storage. Verify auxiliary caches and proxy behavior before declaring containment. [aqua config, S54](https://aquaproj.github.io/docs/reference/config/), [lazy install](https://aquaproj.github.io/docs/reference/lazy-install/)

**uv isolation is packaging isolation, not a sandbox.** Explicitly locate tool environments, executable links, cache, credentials and Python installations; disallow Python downloads unless separately authorized. Check discovered configuration, indexes and build backends. A Portable tool directory alone does not constrain every write. [uv environment, S41](https://docs.astral.sh/uv/reference/environment/), [storage, S55](https://docs.astral.sh/uv/reference/storage/)

The brief's authorized staging/verify/smoke-test/atomic activation/rollback transaction remains the design direction. TUF is a reference for root trust, expiration and rollback/freeze defenses, not evidence that every upstream asset has signed metadata. Reuse aqua's available verification integrations first; add another verifier only for a demonstrated trust gap. Missing upstream signatures require an explicit policy decision, not a self-generated “proof.” [TUF, S42](https://theupdateframework.github.io/specification/latest/), [aqua security, S40](https://aquaproj.github.io/docs/reference/security/)

Use dependency/advisory/license tooling at an approved development stage: RustSec and cargo-deny cover complementary dependency checks. No advisory scan was run because no Cargo project exists. No findings here constitute a vulnerability-free assertion. [RustSec/cargo-deny, S53](https://rustsec.org/)

## Approved tool inventory audit

The [brief](RIVET_PROJECT_BRIEF.md) is the authority for accepted integrations; this stage does not remove or install them. It retains IDs 1–109 but does not contain a complete per-ID manifest. Do not reconstruct missing historical entries from guesses.

| Approved family | Qualification still needed; no current adapter is claimed |
| --- | --- |
| ast-grep, Difftastic, Ruff, actionlint, ShellCheck, fd, Watchexec, Hyperfine, yq, uv | Exact upstream identity/version/license, portability, structured invocation, workspace/config effects and preset membership; Hyperfine benchmark execution requires explicit opt-in. |
| ripgrep standard; fd optional; ugrep/tree non-default; single hex/hash capability | Define overlap and fixture coverage before dropping any selected implementation. |
| jq for JSON; yq for useful multi-format processing | Pin which yq implementation and supported round-trip semantics; name alone is ambiguous. |
| YARA-X preference; TLSH primary/ssdeep on demand; Binwalk first/Unblob fallback; one qualified SquashFS implementation | Rule/format/security/fallback fixtures before deduplication; archive and compression helpers only for actual recipes. |
| checksec, firmwalker, capa, FLOSS; Syft+Grype regular workflow, CVE Binary Tool specialist | Unrivet ownership, permission surfaces, supported input types, evidence reproducibility and tool-specific data-update policy. |
| Ghidra/Rizin complementary; angr advanced optional; optional Ghidra/JDK and morluto/rea | API/runtime/license/footprint qualification in the add-on stage; no sibling repository inspection performed. |
| Difftastic source versus diffoscope binaries/packages | Different comparison domains; one is not evidence that the other can be removed. |

This inventory table preserves acceptance and uncertainty; it is not a source-backed per-tool dependency approval. Exact versions/licenses for the specialist catalog remain a gate in P5/P6 and D12. Broadening P0.5 into an exhaustive 109-entry audit is impossible without the missing authoritative manifest; the current research instead qualifies the architectural choices and records the gap.

## Complexity and duplication audit

| Tempting addition | Recommendation and reason |
| --- | --- |
| Separate message DB, task DB, audit logger service | Use one ledger with typed records and transactional relationships; tracing remains diagnostic only. |
| Workflow daemon per mode or feature | Use one supervisor with mode validation and scheduling; isolate only privileges/crashes/lifecycle. |
| Rig agent runtime plus Rivet loop plus LangGraph checkpointing | Evaluate reuse at one boundary; do not retain overlapping state owners. |
| Custom provider protocol, MCP transport or package manager | Prefer existing libraries/standards/aqua/uv, with explicit policy wrappers. |
| Distributed Mesh infrastructure | Not implied by a peer mode; start with local single-owner task semantics. |
| New model catalog, benchmark daemon, optimizer | Reuse llmfit data, explain budgets; broader optimization remains deferred. |
| Dashboard, Messenger, embedded file manager | Outside approved lean terminal/reporting boundary. |
| Compression as the first size strategy | Avoid unused assets first; measure archive/installed/runtime/peak separately; UPX/SquashFS runtime storage stays experimental and off. |
| Native extension ABI or unrestricted plugin process | Prefer versioned RPC, common registry, broker and receipts; no self-granted privileges. |

## FOSS license options for the owner

No license is chosen or added. This is a comparison of policy tradeoffs, not a legal compatibility clearance. Specific linking, distribution and plugin relationships need review against actual selected artifacts.

| Option | Relevant effect | Tradeoff for Rivet | Primary/explanatory source |
| --- | --- | --- | --- |
| MIT | Permissive reuse with notice retention | Simple adoption, no general requirement to return modifications; no Apache-style express patent clause | [S43](https://opensource.org/license/mit) |
| Apache-2.0 | Permissive, express contributor patent grant/termination and notice/change obligations | Clearer patent terms; more notice administration; preserve applicable NOTICE content | [S44](https://www.apache.org/licenses/LICENSE-2.0) |
| MIT OR Apache-2.0 | Recipient chooses one license; common Rust ecosystem pattern | Permissive flexibility; contributors must knowingly provide both sets of rights | [MIT](https://opensource.org/license/mit), [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) |
| MPL-2.0 | File-level copyleft for distributed covered code | Keeps modified covered files open while permitting larger differently licensed works; file boundaries and notices matter | [Mozilla FAQ, S45](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) |
| GPL-3.0 family | Strong distribution/combined-work source obligations | Stronger reciprocity; extension/linking and dependency compatibility need deliberate review; only/or-later choice is separate | [S46](https://opensource.org/license/gpl-3.0) |
| AGPL-3.0 family | Adds network-interaction source-offer terms for modified covered programs | Relevant to future hosted forks; greater integration review burden; does not make every API client AGPL | [S47](https://opensource.org/license/agpl-3.0) |
| LGPL family | Library-focused reciprocity/relinking obligations | More natural for a separately reusable library than a simple CLI policy; version and static/dynamic distribution matter | [LGPL-2.1 text, S48](https://raw.githubusercontent.com/containers/bubblewrap/main/COPYING) |

The owner's priorities—permissive adoption, contribution reciprocity, patent clarity and future hosted use—must drive D13. Invoking an external tool is different from linking its code, but bundling an executable still carries that executable's distribution obligations. Provider service terms, model weight licenses, Chromium notices and plugin licenses remain distinct from Rivet's source license.
