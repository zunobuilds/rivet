# Rivet

Rivet is a planned standalone AI coding-agent platform for developers who want a terminal-first coding assistant, configurable agent hierarchies, and explicit control over models, tools, permissions, and long-running work. Rust-based development is planned, with useful pre-1.0 milestones leading to a mature Linux release.

**Status: planning and initial documentation only.** This repository currently contains project guidance and a product brief, plus this README and a development `.gitignore`. There is no application implementation, Cargo project, installer, tested integration, release, or benchmark. Every product capability described below is **planned**, unless explicitly identified as present documentation. No installation or runtime commands are available to verify yet.

Rivet is intended to be free and open-source software (FOSS), but no license has been selected or included. The FOSS goal is not a present license grant.

## Contents

- [Current status and getting started](#current-status-and-getting-started)
- [Coding experience](#coding-experience)
- [Agent modes and execution](#agent-modes-and-execution)
- [Multi-day autonomy and communication](#multi-day-autonomy-and-communication)
- [Models and Cookbook](#models-and-cookbook)
- [Browser and desktop inspection](#browser-and-desktop-inspection)
- [Security and permissions](#security-and-permissions)
- [Installation presets and tool ownership](#installation-presets-and-tool-ownership)
- [Packaging, storage, and updates](#packaging-storage-and-updates)
- [Optional Unrivet integration](#optional-unrivet-integration)
- [Roadmap and qualification](#roadmap-and-qualification)
- [Project documentation and participation](#project-documentation-and-participation)
- [License and third-party materials](#license-and-third-party-materials)

## Current status and getting started

| Area | Present state |
| --- | --- |
| Product definition | Recorded in the [project brief](planning/RIVET_PROJECT_BRIEF.md), dated 30 October 2026. |
| Repository guidance | [AGENTS.md](AGENTS.md) defines work boundaries; the [documentation guide](GITHUB_DOCUMENTATION_GUIDE.md) defines documentation practices. |
| Coding agent and runtime | Planned; no runnable software is present. |
| Installation and configuration | Planned; no installer, supported provider configuration, or minimum Rust version is established. |
| Testing and compatibility | No application tests, CI workflows, platform qualifications, or performance results are present. |
| Licensing | Undecided; no license file is present. |

To explore the project today, read this overview and the project brief. There is no working coding-session example or supported source-build procedure yet. A future quick start must be derived from implemented tooling and tested prerequisites, rather than speculative CLI syntax.

## Coding experience

The planned interface is terminal-first chat, broadly in the category of Codex CLI or Claude Code CLI, with Rivet's own agent runtime. It does not require an embedded OpenCode runtime, graphical frontend, file-manager interface, separate Git-management interface, or Messenger interface.

Planned coding capabilities include:

- Project-aware file reading, search, editing, patches, creation, renaming, deletion, and reviewable diffs.
- Terminal commands for builds, tests, linting, debugging, and other authorized development tasks.
- Permission-controlled Git workflows and new-project scaffolding.
- Context and session management for continuing work across tasks and interruptions.
- Model setup, Model Context Protocol (MCP) integrations, skills, and a shared tool registry.
- Browser automation and qualified native desktop inspection as part of development and validation.

A planned workflow is to understand a project, agree on a task and its permissions, make inspectable changes, run relevant checks, and report results with evidence. This is a workflow goal, not an executable example.

Human-facing reporting is intentionally lean: task results, diffs, status, and exportable Markdown or JSON, backed by required audit logs and checkpoints. A separate resource dashboard or report designer is outside the current design.

## Agent modes and execution

The following cardinalities are requirements for the planned modes. Role names describe agent responsibilities; they do not require a distinct loaded model for every agent.

| Mode | Required composition | Intended shape |
| --- | --- | --- |
| **Solo** | Exactly 1 **Independent** | One coding agent; the only mode in Rivet Minimal. |
| **Pair** | Exactly 1 **Lead** and 1 **Operator** | A lead coordinates an operator. |
| **Crew** | Exactly 1 **Lead**, 1 **Reviewer**, and at least 1 **Operator** | Coordinated execution with a dedicated review role. |
| **Fleet** | Exactly 1 **Director**, at least 1 **Lead**, at least 1 **Reviewer**, and at least 1 **Operator** | A director coordinates a larger hierarchy. |
| **Mesh** | Configurable peers | Precise ownership and scheduling semantics remain to be designed. |
| **Adaptive** | Configurable selection of valid modes | Mode selection must preserve each selected mode's fixed-count rules. |

Temporary specialists must fit the active mode's invariants or request a permitted mode change. Adaptive selection cannot silently bypass those constraints.

Three configurable execution behaviors are planned:

| Behavior | Design intent |
| --- | --- |
| **Concurrent** | Execute agents concurrently where provider capacity, resources, and permissions allow. |
| **Shared Model** | Allow multiple agent roles to use shared model capacity instead of requiring one loaded model per role. |
| **Model Relay** | Support staged use of model resources across agents when simultaneous execution is unsuitable. |

Exact concurrency, relay, process, and scheduling contracts still require implementation and qualification. Model selection, installer preset, agent mode, and execution policy are independent choices, subject to the installed capabilities and feasibility checks.

## Multi-day autonomy and communication

Rivet aims to support genuinely multi-day work within explicit user authorization. The design calls for recoverable checkpoints, replay and idempotency protections, pause and stop controls, configured limits, and robust cancellation. Restarting must not blindly repeat side effects. These are recovery requirements, not current guarantees.

The proposed runtime uses one supervisor, one trusted permission broker, one shared tool registry, and one durable task/message/audit ledger. SQLite is the principal ledger candidate; its schema and durability contracts remain open. Separate daemons, schedulers, message databases, loggers, or research gateways should not be introduced for every feature. Process isolation is reserved for justified privilege, crash, or lifecycle boundaries.

Human communication stays in the main terminal chat. Planned durable inter-agent messaging includes owner/leader routing, acknowledgments, evidence references, and consent for forwarding important user decisions. Human edits to project files are informational context, not elevated instructions or implicit permission grants.

## Models and Cookbook

### No default model downloads

**No Rivet or Unrivet preset may preselect, download, or install AI model weights by default.** Selecting a preset must not automatically install a local inference runtime such as Ollama or llama.cpp. No particular model, including a Qwen 30B model, is compulsory.

The user chooses an existing compatible local provider or remote endpoint, or explicitly elects to acquire a runtime and model. Local and cloud providers are optional alternatives. A coding inference session will require a compatible configured provider, but model-free setup, configuration review, repair/status, and manual or offline administrative functions are planned to remain usable.

An absent model or missing credentials must result in an honest status such as “provider not configured.” It must not trigger an automatic download, cloud fallback, upload, paid call, or concealed account setup. Inference readiness must be checked before being claimed.

Permitted local-to-cloud or cloud-to-local switching will be configurable, with cost, quota, privacy, and data-transfer warnings. Remote inference can incur provider charges and transfer submitted project context; switching requires the user's permitted policy. Provider compatibility, credentials, configuration formats, and runtime support are not yet implemented.

### Cookbook recommendations

Cookbook is the planned recommendation layer for model choices, role allocations, and resource budgets, with **Efficient**, **Balanced**, and **Ambitious** priorities.

The design favors reusing qualified `llmfit` hardware and model-fit data where useful. Rivet will assess feasibility for the whole hierarchy, including local resource demands, remote budgets, serial queues, and model relay, and explain slow or unsupported configurations. It should not build a redundant model catalog merely to reproduce existing fit data.

Broader automatic optimization is deferred, rather than required for the first release. Benchmarks require explicit user authorization. No model-fit, concurrency, speed, cost, or quality measurements are claimed today.

## Browser and desktop inspection

**Playwright with Chromium belongs to Rivet**, independently of Unrivet. Planned uses include browser automation, web application inspection, and testing development changes.

Qualified **Linux native application inspection** is also planned, using suitable accessibility interfaces, desktop portals, and native APIs with consent. This is distinct from browser testing; Playwright does not by itself establish native desktop support. Linux Wayland permission constraints and actual application coverage remain to be qualified.

The release direction is Linux first. Post-1.0 macOS support must include both Playwright and actual macOS native application inspection, with Windows following later. None of these platform integrations is implemented or certified in this repository.

## Security and permissions

The planned security model centers on explicit project/task permissions enforced through a trusted broker and recorded in the durable ledger. Installing a tool, selecting a preset, or enabling an add-on must not silently authorize its use.

Planned boundaries include:

- Permission-controlled file, terminal, Git, browser, desktop, and add-on operations, with consent appropriate to the task and capability.
- Separate project data and permissions even when immutable tools are shared between installations or projects.
- Authorized model acquisition, external calls, data transfers, benchmarks, and update policies.
- Reviewable changes, durable audit evidence, checkpoints, limits, and cancellation during autonomous work.
- Verification of tool provenance, platform compatibility, and update integrity before activation.

These are design requirements, not evidence of an implemented sandbox, security audit, compliance certification, or complete threat model. Credential storage and isolation contracts still need qualification. No security reporting contact or verified private vulnerability reporting route has been established in this repository; do not publish secrets in issues or reports.

The development `.gitignore` excludes common local secret files and artifacts. Ignore rules do not enforce runtime permissions, detect every secret, or remove files already tracked by Git.

## Installation presets and tool ownership

Rivet and Unrivet are planned to have independent **Minimal**, **Recommended**, **Full**, and **Custom** presets. Their exact package membership and platform dependencies remain to be qualified.

| Rivet preset | Planned scope |
| --- | --- |
| **Minimal** | Useful Solo coding; no hierarchy modes. |
| **Recommended** | Hierarchies, Playwright, and qualified Linux application inspection. |
| **Full** | A broader qualified capability set, without installing every catalog entry. |
| **Custom** | User-selected qualified capabilities for the project and environment. |

**Portable is always the recommended installation location.** System installation is explicit opt-in and is never the recommended location. There is no Hybrid location mode. Presets do not bundle AI models or automatically acquire inference runtimes.

Tool acquisition should first reuse verified available installations consistent with the selected location mode, offer an authorized managed version when appropriate, or leave the tool absent. Additional tools are selected by project, preset, and task type; there should be no unwanted package installations.

### Coding tools and capability selection

Accepted planned Rivet specialist CLI adapters include **ast-grep, Difftastic, Ruff, actionlint, ShellCheck, fd, Watchexec, Hyperfine, yq, and uv**. Invocation and installation are language- and task-aware; acceptance is not proof of an implemented adapter or inclusion in every preset.

The historical inventory retains IDs **1–109** for provenance. It includes models, libraries, system facilities, and custom obligations as well as tools; it is not a list of 109 independent applications to install. The current brief does not provide a complete per-ID manifest.

The planned deduplication policy favors:

- ripgrep for standard search, with fd optional and ugrep/tree non-default; a single hex/hash capability implementation.
- jq for JSON and yq for useful multi-format operations.
- Difftastic for source diffs; diffoscope for binary/package comparison when the relevant capability is selected.
- Language-specific lint tools only where relevant, and Watchexec/Hyperfine only when useful.
- Archive and compression helpers only where actual recipes require them.

Specialist reverse-engineering tool choices are described under [optional Unrivet integration](#optional-unrivet-integration). Tools with overlapping purposes are not assumed interchangeable: formats, fixtures, security, and fallback coverage must be validated before removing an implementation.

## Packaging, storage, and updates

The planned packaging strategy begins by avoiding unused assets and installing optional capabilities on demand. Matching immutable tool/dependency copies may be shared across Rivet, Unrivet, and projects within an application-managed Portable root, while keeping project data and permissions separate.

Qualified upstream slim distributions and justified deterministic source-build or debug-stripping options may reduce footprint. Download archives may use zstd/xz where supported, but extraction restores normal installed size. Required licenses, notices, runtime modules, and security receipts must be retained. Original research evidence bytes must never be modified for storage optimization.

**aqua and uv are the preferred existing tools to prototype**: aqua for selected pinned upstream CLI release assets verified as portable and supported, and uv for isolated Python tools. Rivet does not plan a bespoke general-purpose package manager. Their ability to obey the Portable root and trust policy remains to be validated; fallback acquisition needs separately justified policy.

### Proposed update transaction

1. Under an authorized policy, check known original upstream projects and show version, license, footprint, and changes.
2. Download to staged application-owned storage and verify checksums plus available trusted signatures or attestations, exact operating system/ABI compatibility, and dependencies.
3. Smoke-test without project access; optionally create and reproducibly optimize a derivative artifact where justified.
4. Atomically activate a pinned immutable version when no task is using the old version, retaining rollback.

Missing upstream signatures must not be invented, and self-generated checksums alone do not establish authenticity. Updates to vulnerability databases, rules, model weights, and plugin APIs have separate compatibility and consent boundaries. There must be no unapproved automatic updates.

UPX and compressed SquashFS runtime storage are **experimental and disabled by default**. Portability, startup time, memory sharing, security tooling, and verification can be affected. Each transformed binary needs independent testing after an upstream update. Downloaded, installed, runtime, and peak/cache sizes must be measured separately; no numerical savings are established.

## Optional Unrivet integration

Unrivet is the planned separately maintained reverse-engineering and security-research extension. **Rivet must work without Unrivet, and removing Unrivet must not break Rivet.** This repository defines Rivet's side of that boundary; it does not contain or verify Unrivet's implementation.

Planned Unrivet capabilities include firmware and binary analysis, specialized device/network analysis, reproducible evidence and provenance reports, and optional `morluto/rea`. Unrivet should reuse qualified installed Rivet tools and the same agent, model, and hierarchy runtime without duplicating assets. Each add-on tool still requires its own project/task permission.

When Unrivet is installed, the planned new-project flow asks **“Enable Unrivet for this project? Yes/No”** and saves an editable per-project setting. It must not auto-enable reverse-engineering, device, or network capabilities. Without Unrivet, Rivet simply creates a Rivet project. Unrivet's tool-pack presets remain independent of Rivet's.

Accepted planned Unrivet CLI adapters include **checksec, TLSH, firmwalker, capa, FLOSS, and ssdeep**. The broader capability selection policy is:

| Capability | Planned preference and qualification boundary |
| --- | --- |
| Rule matching | Prefer YARA-X over original YARA, subject to rule compatibility. |
| Similarity | TLSH primary; ssdeep on demand. |
| Firmware extraction | Binwalk first; Unblob fallback; qualify format and fallback fixtures. |
| SquashFS | Select one qualified implementation according to format. |
| ELF hardening | checksec supplies the original ELF-hardening inventory capability. |
| Software bill of materials and vulnerabilities | Syft + Grype for the regular workflow; CVE Binary Tool as an optional specialist. |
| Binary analysis | Ghidra and Rizin are complementary; angr is advanced and optional. |
| Optional dependencies | Ghidra/JDK, REA, capa, FLOSS, and angr only for qualified selected features. |

These are integration plans, not installed dependencies or working functionality. Two-repository API compatibility and release tests remain open. Unrivet's specialist evidence reporting does not require adding a separate reporting dashboard to Rivet.

## Roadmap and qualification

Small, useful pre-1.0 milestones are expected. **Rivet 1.0.0 is intended to be a mature Linux-only release**, rather than the first feature-rich prototype. macOS follows with browser and native inspection, then Windows. Dates and exact intermediate release numbers are not committed.

The proposed engineering order is:

| Stage | Planned focus |
| --- | --- |
| **P0** | Authoritative specification and repository setup; this documentation stage is an initial part of that work. |
| **P1** | A functional Solo coding agent. |
| **P2** | Durable recovery. |
| **P3** | Hierarchies, shared-model execution, and model relay. |
| **P4** | Playwright and Linux native application tests. |
| **P5** | Optional Unrivet integration, project opt-in, and reverse-engineering fixtures. |
| **P6** | Updater, shared storage, and measured size optimization. |
| **P7** | Linux v1.0 qualification. |

This is a proposal for sequencing, not a record of completed stages or authorization to implement them. Each future implementation stage requires explicit approval of scope and external effects, evidence-backed checks and reporting, a durable checkpoint, and then a stop until the next stage is approved.

Open qualification work includes exact upstream versions, dependencies, notices and footprints; Portable acquisition and trust enforcement; YARA-X and extraction compatibility; process and ledger contracts; model concurrency and relay benchmarks; Linux Wayland permissions; peer/Adaptive scheduling; add-on API compatibility; macOS and Windows support; signing, attestations and rollback; and name, trademark, and license choices. Broader Cookbook optimization remains deferred.

## Project documentation and participation

- [Project brief](planning/RIVET_PROJECT_BRIEF.md): the authoritative planning reference for this overview, including product boundaries and unresolved decisions.
- [Repository instructions](AGENTS.md): authorized filesystem scope, staged work rules, and Git permissions for agents working here.
- [GitHub documentation guide](GITHUB_DOCUMENTATION_GUIDE.md): evidence-first documentation and repository practices.

There is no implemented contributor setup, test suite, or established support policy yet. Proposed work should be grounded in the brief and clearly separate design decisions from verified behavior. Do not install tools, acquire models, or start a later implementation stage solely because it appears on the roadmap.

## License and third-party materials

Rivet's intended FOSS licensing remains an owner decision. No license is included, and this README does not select one. Third-party tools and models retain their own licenses and terms; naming a planned integration does not imply bundling, endorsement, or verified compatibility. License notices and acquisition requirements must be qualified before distribution.
