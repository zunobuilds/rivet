# Security and recovery design

**P0.6 authority note:** The [master implementation plan](MASTER_IMPLEMENTATION_PLAN.md) selects the current stack, defaults and contracts; [decision dispositions](open-decisions.md) distinguish owner approvals from technical recommendations. P0.5 comparisons and proposal wording below are retained as research provenance, not competing current defaults. No implementation is authorized.

**P0.5 proposal, not an implemented security guarantee.** The [brief](RIVET_PROJECT_BRIEF.md) approves explicit permissions, a trusted broker, one durable ledger, recoverable multi-day work and cancellation. Mechanisms below require approval and qualification. [Architecture](architecture.md) owns component contracts; [agent execution](agent-execution.md) owns hierarchy and scheduling; [open decisions](open-decisions.md) records unresolved policies.

## Threat model and trust boundaries

Protect project files, unrelated host data, credentials, model/provider budgets, desktop sessions, original research evidence and the integrity of user decisions. Treat repositories, build scripts, Git configuration, dependency installers, browser pages, screenshots, tool output, MCP servers, extensions and peer-agent summaries as potentially hostile input. A model is not a policy enforcement point.

Propose trusting the supervisor/broker/ledger implementation and qualified OS enforcement, while isolating execution workers. A compromised host administrator or compromised kernel is outside the initial boundary. Same-user host malware may read application state or tamper with binaries; file permissions alone do not establish an adversarial same-user security boundary. Scope any future security claims to the actual tested profile.

A permission grant should bind actor/run, semantic operation, exact tool identity, arguments, resources/destination, project revision where relevant, effect class, expiry, limits and policy epoch. The broker rechecks it at dispatch. Model text, a tool's self-description, a specialist role or a previously successful command cannot expand the grant. Approval prompts must show consequential effects in human-readable form without terminal escape injection.

## Permission matrix

| Surface | Proposed default and scope | Important bypasses and controls |
| --- | --- | --- |
| Filesystem | Explicit root capabilities; separate read/write/create/delete grants; protected application state | Reject traversal, escaping symlinks/magic links, unsafe mounts and stale write preconditions; do not authorize using a string prefix check. |
| Git | Read inspection separate from index/ref writes, history rewriting and remote operations | Hooks, filters, external diff, signing and credential helpers can execute programs. Push needs destination/ref/effect-specific consent; a commit permission is not push permission. |
| Terminal/build/test | Commands are process execution with executable/argv/cwd/environment and resource scope | Repository scripts, compilers and tests can execute arbitrary code. Use sandbox restrictions rather than pretending command names make them safe. |
| Network | Deny ungranted egress; provider, research, package, project-server and device traffic are separate capabilities | Redirects, DNS rebinding, loopback services, metadata addresses, IPv6, proxies and DNS itself need enforcement. A domain allowlist in an HTTP client does not restrict child processes. |
| Browser | Explicit origins/actions, isolated profile and granted artifact destinations | Page subresources, redirects, WebSockets, downloads/uploads, service workers, extensions and browser debugging ports can exceed a navigation-only policy. |
| Desktop | Target application/window plus separate tree read, screenshot, input and clipboard grants | Accessibility buses and remote-desktop grants can be broader than one app. Cross-window focus changes, file pickers and session revocation need checks. |
| MCP/add-ons | Explicit server/tool/version and effects; Unrivet project opt-in plus per-tool permission | Tool descriptions/results are untrusted. Dynamic schema/capability changes invalidate approvals; no token passthrough or extension-created grants. |
| Model/provider | Explicit endpoint, data scope, credentials and spending policy | Local endpoints can route to cloud; compatibility does not imply privacy. Do not auto-fallback, upload context, download weights or benchmark. |

Git hooks are documented executable integration points, so Git must be treated as more than a file-format API. Qualification must decide how to suppress unapproved helpers while preserving requested repository behavior. Do not silently disable user hooks and then claim the normal workflow passed. [Git hooks, S16](https://git-scm.com/docs/githooks)

## Linux isolation options

Recommend qualified bubblewrap namespaces plus a minimal filesystem view for untrusted commands, with Landlock as defense in depth where supported, syscall restrictions where justified, and cgroup resource enforcement where delegated. This is a candidate profile, not an approved runtime dependency or installed capability. bubblewrap explicitly leaves policy to its caller; exposing a broad D-Bus socket can defeat intended restrictions. [bubblewrap, S29](https://github.com/containers/bubblewrap)

Landlock is ABI-dependent; probe required rights and fail closed for operations that cannot be contained. Report unsupported enforcement rather than silently continuing with a weaker sandbox. Its network controls do not implement hostname-aware egress. [Kernel Landlock, S30](https://docs.kernel.org/userspace-api/landlock.html)

Descriptor-relative filesystem APIs such as cap-std and Linux `openat2` resolution constraints can strengthen built-in file operations. Canonicalizing once and later opening a path is race-prone. Revalidate target identity/preconditions at mutation; handle rename/delete and hard links separately. Disallow unqualified special files and cross-boundary links. Built-in safe APIs do not constrain arbitrary external executables. [cap-std, S13](https://docs.rs/crate/cap-std/latest), [openat2, S31](https://man7.org/linux/man-pages/man2/openat2.2.html)

A namespace with no network is a useful narrow starting profile. Granted egress likely needs a brokered proxy/transport that validates every connection and blocks bypasses; direct raw-network/device research is a distinct high-authority capability. Exact DNS, UDP, inherited socket and proxy behavior remains D05. Do not expose the broker, ledger, credentials, host process control or entire session bus to workers.

Containers alone are not a complete security policy. gVisor's application-kernel approach is an alternative for stronger separation, with compatibility and operational costs that need measurement; it is not a replacement to assume for browser/desktop workloads. Virtual machines are another possible future boundary for high-risk analysis, outside the initial lightweight profile. [gVisor security architecture, S52](https://gvisor.dev/docs/architecture_guide/intro/)

## Browser and desktop constraints

Recommend a pinned official Playwright adapter in a separate worker and a qualified Chromium sandbox. Browser contexts separate browser state; they are not host isolation. Do not solve sandbox failures by silently adding `--no-sandbox`. Upstream's Docker guidance distinguishes trusted testing from untrusted browsing and explains the root-user issue. [Playwright security guidance, S35](https://playwright.dev/docs/docker)

Separate automation permission from permission to transmit content or make changes on remote sites. Bound screenshots/traces/downloads and redact before model transfer/export. Avoid personal browser profiles and cookies by default; authenticated workflows require specific consent. Page content cannot approve tool use. Navigation interception must be tested against non-navigation traffic and redirects before claiming egress enforcement.

Use AT-SPI for semantic native inspection and XDG portals for separately consented screen/input capabilities. Neither guarantees every application exposes useful structure. A portal can grant broad input access; narrow target checks must remain in Rivet. Stop input when window identity/focus changes unexpectedly or a portal session closes. Headless environments and denied portals should produce honest capability errors. [AT-SPI, S36](https://docs.gtk.org/atspi2/), [RemoteDesktop portal, S39](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)

## Prompt injection and instruction provenance

Repository files, web pages, issue text, tool output and screenshots can contain adversarial instructions. Preserve provenance/trust labels through extraction, messages and compaction. Treat skills/MCP prompts as untrusted until explicitly configured under policy. Human file edits do not become approval messages. Avoid automatic execution merely because content resembles an instruction file.

Recommended controls: trusted instruction channel separate from evidence; least-privilege grants outside the model; tool schema validation; restricted output rendering; data minimization; explicit sensitive egress approval; and adversarial fixtures across file/web/peer channels. Detection or a warning prompt is supplemental, not proof of prevention. OWASP notes that retrieval and fine-tuning do not eliminate this risk. [OWASP LLM01, S33](https://genai.owasp.org/llmrisk/llm01-prompt-injection/)

For remote MCP, validate authentication audience/scopes and destinations; never forward arbitrary tokens to another service. Local MCP processes still execute software and need the same containment as commands. Server names, session IDs and tool annotations do not themselves establish trust. [MCP security practices, S22](https://modelcontextprotocol.io/docs/2025-11-25/tutorials/security/security_best_practices)

## Credentials, privacy and evidence

Recommend an OS credential-store adapter with opaque references in configuration/ledger, plus an explicit ephemeral input/environment option for headless use. Never silently persist a plaintext fallback when the keyring is unavailable. keyring provides reusable backend abstractions; backend/session availability remains a platform qualification task. Secret Service's fetched latest specification is a draft, so do not assume its newest features exist on a target desktop. [keyring, S12](https://docs.rs/crate/keyring/latest), [Secret Service, S50](https://specifications.freedesktop.org/secret-service/latest/)

Give workers only needed credentials through narrow channels; scrub unrelated inherited environment and close unnecessary descriptors. Avoid secret-bearing argv, debug dumps, raw HTTP recordings and automatic crash uploads. Encryption at rest and audit tamper-evidence are open decisions; SQLite by itself should not be described as encrypted or tamper-proof.

Retain authorization metadata and redacted operational evidence. For original Unrivet evidence, preserve bytes and use access-controlled artifact storage; produce redacted derivatives for sharing without altering originals. Audit-retention, user deletion and privacy policies must distinguish required metadata, optional transcripts and original evidence. No automatic cross-project memory, telemetry or cloud upload is proposed. Hashes detect accidental mismatch, not authenticity against an attacker who can replace both content and hashes.

## Effect classes and replay

A database commit cannot atomically include an arbitrary filesystem, process or remote action. Promise durable intent, receipts and reconciliation, not universal exactly-once execution. [SQLite atomic commit, S28](https://www.sqlite.org/atomiccommit.html)

| Effect class | Examples | Proposed recovery rule |
| --- | --- | --- |
| Observation | Bounded file read, status query | May repeat within current grants; record revision/time because observations can change. |
| Conditional local mutation | Patch with expected hash, atomic replace | Inspect target/precondition/receipt; retry only if non-application is established, accept matching post-state, otherwise conflict. |
| Transactional local state | Task claim, message acknowledgment, budget reservation | Deduplicate stable operation/event IDs in one ledger transaction. |
| Non-idempotent command | Arbitrary build script, commit hook, device write | Treat a missing final receipt as unknown; inspect/reconcile or require human decision. |
| External action | Push, upload, provider request, site action | Use remote idempotency keys only where actually supported; query request/effect state where possible; never blind retry an ambiguous outcome. |

Separate stable operation IDs from per-attempt IDs. Persist intent and budget/grant binding before dispatch. Completion records should include before/after observations and artifact references sufficient to distinguish done/not-done/conflicted/unknown. A local outbox makes dispatch recoverable but does not make a remote endpoint transactional with SQLite.

## Proposed operation lifecycle and crash windows

Proposed lifecycle: proposed → authorized → intent-recorded → dispatched → succeeded/failed, with reconciling/outcome-unknown states for ambiguous transitions. Cancel-requested is a control state, not proof of no effect. A task cannot succeed while a required operation remains unknown.

| Crash window | Required behavior on resume |
| --- | --- |
| Before durable intent | No dispatch is allowed; discard incomplete proposal and revalidate grants. |
| After intent, before known dispatch | Do not assume no effect: worker launch may have happened before the dispatch marker. Reconcile using process/effect identity. |
| After effect, before durable receipt | Mark unknown and inspect post-state or remote request status; require decision when proof is unavailable. |
| After receipt, before task transition | Deduplicate receipt/operation ID and finish ledger transition without redoing the effect. |
| During message delivery | Redeliver with deduplication; acknowledgment cannot be taken as proof that a separate external effect completed. |
| During artifact/checkpoint creation | Accept only durable verified artifact references; quarantine incomplete blobs; do not use partial context as a complete checkpoint. |
| During migration/update | Restore a verified compatible backup or retain old immutable version; never guess a newer schema is readable. |

For artifacts outside SQLite, propose write-to-staging, flush, content hash, atomic rename on the same filesystem, directory durability step, then ledger reference. Recovery can collect orphan blobs after verifying that no live operation references them. SQLite and artifact storage do not share a transaction; this protocol needs power-loss and disk-full qualification on supported filesystems.

## Checkpoints, restart and multi-day work

A checkpoint should record schema/version, session/run/task cursors, hierarchy/ownership epoch, grant references, budget reservations, pending operations/messages, provider/model configuration, tested workspace revisions, artifact integrity and context-source references. Do not serialize credentials or assume arbitrary process memory is portable.

On restart: acquire exclusive project supervision; verify schema/database/artifacts; identify and contain old process trees using stable identities; fence previous owners; reconcile ambiguous operations; recompute deadlines; recheck grants and budget; present recovery summary; resume only work still authorized. PID alone is unsafe because of reuse. Expired grants and portal sessions must not renew automatically.

Use an online database backup mechanism and restore drills; copying the live main database without WAL considerations is insufficient. Bound WAL growth and disk use. Local WAL restrictions mean a network-shared project root cannot silently become a supported ledger location. [SQLite WAL and backup, S27–S28](https://www.sqlite.org/wal.html), [backup API](https://www.sqlite.org/backup.html)

Multi-day work requires an explicit lifecycle policy for terminal disconnect, suspend/reboot, credential expiry, provider outage, disk exhaustion and user unavailability. Propose pause-on-disconnect initially; later an explicitly authorized detached mode can reuse the same supervisor rather than invent a second scheduler. Durability permits recovery after downtime; it does not guarantee continuous execution while the machine is asleep.

## Cancellation and limits

Cancellation must be out-of-band from inference and must not wait behind a full ordinary work queue. Propagate run/task cancellation tokens; stop new dispatch; send cooperative worker/provider cancellation; enforce a deadline; terminate and reap the permitted process tree; record remaining/unknown effects. Closing an HTTP stream may not cancel provider-side work or billing. Revocation after dispatch cannot undo completed effects.

Tokio explicitly documents that dropping a child handle does not by default terminate the process and that reaping matters. Thus task abortion alone is insufficient. Process groups, namespace cleanup and delegated cgroups should be qualified together; detached descendants must not evade cancellation. [Tokio processes, S49](https://docs.rs/tokio/latest/tokio/process/index.html), [cgroup v2, S32](https://docs.kernel.org/admin-guide/cgroup-v2.html)

Bound process count, output buffers, disk, wall time, CPU and memory where enforcement is available. If a required controller is unavailable, explain and deny that execution profile rather than claim its limits are enforced. The shutdown sequence should flush critical state and await owned workers; hard termination still enters recovery. [Tokio shutdown, S02](https://tokio.rs/tokio/topics/shutdown)

## Required future evidence

Before security/recovery claims: adversarial path/link/mount and credential fixtures; Git helper/hook tests; egress bypass tests; injection through tools/files/pages/peer summaries; permission revocation races; malformed plugin frames; orphan descendants; bounded cancellation under load; interruption at every crash window; disk-full/corrupt artifact/restore/migration cases; user edits during recovery; clock changes and expired grants; required bounded crash/restart, fault-injection and stress qualification per [the current contract](P2_P3_QUALIFICATION.md); optional multi-day endurance at P7. No real host reboot/suspend/power/clock change is implied. These are proposed tests, not checks run in P0.5.
