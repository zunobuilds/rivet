# Product, official mods and hybrid terminal UX

Research only. The [owner brief](../POST_P3_RESEARCH_BRIEF.md) approves Solo + Simple default, optional native hierarchies, chat-first hybrid UI and one global Command Center. It does not approve command names, layouts, shortcuts, third-party loading or implementation here. This supersedes older blanket exclusions of a dashboard: the approved dashboard is a compact terminal view, not a separate reporting product/service.

## Product and mod lifecycle

Keep one useful Independent agent and ordinary chat as the first screen. Pair/Crew/Fleet/Mesh/Adaptive remain built-in optional features. No hierarchy is packaged as a mod. Cookbook can propose mode/provider/count/intensity with an explicit user override and hard permission/budget ceilings. A temporary Solo helper remains a proposal: require a permitted composition/delegation transition, explicit ceiling and scoped task, preserving Solo semantics when helper work ends. No unbounded hidden spawning.

Unrivet is the first official owner-maintained optional mod; Rivet remains useful without it. Initial loading should allow only vetted official identities, not arbitrary third-party code. Proposed manifest: stable namespaced ID, manifest/API version, exact artifact digest, publisher/provenance, licenses/notices, compatible Rivet range, dependency identities, declared tools/roles, effects/capabilities, storage and lifecycle requirements. Installation, enablement and per-operation permission are three separate decisions. A mod may augment existing agents or supply a specialist role within core count/grant rules.

Proposed state transition: enabled → disable-requested → no new admissions → revoke affected permissions/cancel or safely drain bounded work → record checkpoints/unknown effects → remove all exposed UI/agent/MCP/tool registrations → disabled. Retain installed bytes/settings, but no workers, timers, scans, update polls or model preloads. Re-enable requires explicit user action and current manifest/grant validation. If stopping fails, show disabling/failed rather than claiming operational absence. Uninstall changes reference ownership; dependencies still used by Rivet or another owner survive. Deny forced dependency deletion while leased.

Official optional **Rivet-Bench** should contribute fixture recipes and result schemas through the same boundary. Cover startup, idle, inference, scheduling, tools, Git, recovery and multiple projects. Each run records version/config/OS/runtime/model/dataset hashes, warm/cold state, sample count, raw observations and failures. Synthetic tests and live measurements are distinct. Live inference/paid calls require their own approval. Disabled means zero unnecessary background work, not an idle benchmark daemon. Acceptance: disabled catalog/commands/MCP descriptors absent, no child/timer activity, settings preserved and enable/disable races safely fenced.

## Research references and proposed navigation

[Codex CLI features](https://developers.openai.com/codex/cli/features) and [Claude Code CLI](https://code.claude.com/docs/en/cli-reference) illustrate interactive continuation and session selection. [Claude permissions](https://code.claude.com/docs/en/permissions) illustrates explicit modes/rules. Adopt discoverability and visible scope, not another product's permission defaults. This is a documentation study, not execution or a compatibility claim.

| Proposed command | Proposed role and safety rule |
| --- | --- |
| `/projects`, `/workspace` | Find explicitly authorized projects; switch arbitrary logical groups. Candidate discovery is not read permission. |
| `/sessions` | Show independent session identity/state; attach locally after ownership checks. |
| `/dashboard`, `/agents` | Expand global Command Center, inspect task/role progress and conflicts. |
| `/model` | Review inherited/overridden provider and budget/capacity scope; never silently acquire/switch. |
| `/permissions` | Inspect grants and approval inbox; deny/revoke immediately through trusted broker. |
| `/status`, `/resources` | Compact lifecycle/budget summary; Advanced adds local resource estimates and uncertainty. |
| `/diff`, `/history` | Project-scoped revisions, receipts and agent-attributed changes. |
| `/undo`, `/recycle` | Preview selective reversal or restore, conflicts and new permissions before mutation. |
| `/pause`, `/detach` | Distinguish resumable quiescence from leaving bounded work attached to core. |
| `/quit` | Confirm Safe Quit scope/deadline, preserving completed and incomplete records. |
| `/settings` | Simple settings first; Advanced diagnostics/overrides with clear effective scope. |

Names remain proposals; current executable has ordinary subcommands, not these slash commands. “Duo” appears in the brief's future session/notification vocabulary; existing implemented two-agent mode is Pair. Do not silently rename Pair or equate a future Duo session model with Lead/Operator semantics; owner decision needed.

One global Command Center projects views from all authorized workspaces/projects/sessions. Workspace filters change presentation only. Show project and session identity persistently; allow name collisions without ambiguous IDs. Suggested wide layout: chat with optional status/approval pane; narrow layout: stacked one-pane views and compact status line. Palette and slash search share one action registry, with unavailable/disabled capabilities absent. Remappable keybindings show conflicts and have reset/export. Avoid assigning final shortcuts before terminal/accessibility tests.

Background approvals increment a visible inbox count without moving focus. Each card shows trusted project/root alias, session, agent, exact operation/destination/data/effect, expiry and budget. Offer explicit deny and narrow one-shot/session/project grants only where the operation supports them; project grant is not cross-project authority. Changing active workspace cannot change an approval's target. Agent-originated requests can never approve their own escalation. Stale/revoked cards expire atomically; duplicate replies are idempotent.

## Acceptance design

Test keyboard-only operation, screen-reader/plain output, no-color/contrast, long paths, Unicode/wide glyphs, rapid resize, small terminals, paste and terminal escape injection, focus retention and key remapping. Verify progress and approval input remain responsive during slow tools/ledger activity. Closing a view must not kill or authorize unrelated sessions. Reconnect shows durable current status and missed events, never trusts cached UI as authority. Versioned UI requests route to each project supervisor; the global UI is not a superuser broker. No remote UI is planned now.
