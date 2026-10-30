# Workspaces, sessions and trusted coordination

Research only. Workspace model C is approved: folder-based candidate discovery plus arbitrary logical grouping of projects at unrelated paths, stable IDs, custom renameable names and many-to-many project membership. Discovery is not permission. A workspace is a view/group, not a permission boundary; one project record/ledger persists across its workspace memberships.

## Isolation and concurrent writing

Each project owns authorized roots, ledger, memory, settings, approvals, budgets, provenance and audit. Each concurrent Solo/Duo/hierarchy session owns private conversation/context, task, grants and lifecycle. Shared provider weights never imply shared conversational KV state. Joining a workspace cannot expose another project's files or retrieved memory. Global UI only queries authorized metadata.

| Writer approach | Benefits | Risks / proposed use |
| --- | --- | --- |
| Shared directory + file/Git leases | Low disk overhead; immediate integrated view | Leases constrain cooperating agents only. Noncooperating human edits race; before/after revisions, compare-and-set writes and serialized Git index/ref lane remain required. Suitable for read-heavy or strictly partitioned work. |
| Independent Git worktrees/branches | Independent index/workspace/branch per writer; inspectable integration diff | Shared repository objects/refs and common config still require coordination. Untracked/ignored assets may be absent; builds consume extra disk. Integration conflicts and human changes need explicit review. |

Recommend worktrees/branches for concurrent writers, **not yet owner-approved**. [Standard Git worktrees](https://git-scm.com/docs/git-worktree) provide linked working trees; they are not security sandboxes. Grant each writer only its tree, restrict shared Git mutations and pin integration base/revisions. Integrate through reviewed patches/commits after checking the destination's current state. Never reset a shared branch to undo one agent. Selective reversal computes that agent's attributable diff, checks later edits, and either applies a conflict-free inverse or requests resolution. No reliable authorship can be inferred from Git author alone.

## Broker protocol and delivery

Prefer in-process typed channels for projects already under one core, private Unix sockets for independent trusted supervisors. Avoid public listeners and a daemon per workspace. Local peer credentials establish OS identity but same-user peers still need private channel/owner-generation checks; filesystem permissions alone are not hostile same-user isolation. Versioned envelopes carry runtime-assigned sender/recipient project/session/agent IDs, request/message IDs, type, scope/grant, epoch, deadline, size, provenance and content/artifact digest. Never accept self-declared JSON agent identity as authority.

Explicit cross-project policies separately allow (1) messages, (2) selected artifacts and (3) delegated task requests. Message permission does not grant file reads or task execution. Artifact transfer is an explicit selected immutable copy/reference with classification and recipient data grant; never an ambient shared mount. Receiving supervision accepts/rejects independently, schedules under its own permissions/budget and acknowledges only durable admission. Untrusted sender text cannot change local instructions or import model context.

Durable outbox/inbox, stable deduplication IDs, bounded payload/queue, deadlines, receipts and revocation support pause and crash recovery. Receipt means delivered/admitted, not task succeeded. Paused projects retain queued messages without execution; expired/revoked entries are withheld with audited reason. Delegation acceptance is a distinct transition, not automatic execution on receipt. Resource requests follow the same trusted supervisor route and lease/epoch/idempotency/fencing discipline described in [resources](post-p3-resources.md).

Test replay, altered duplicate, wrong identity/project, revoked grant, recipient offline/pause, crash before/after durable acknowledgment, deadline expiry, artifact substitution and confused-deputy cleanup. Audit source and recipient decisions without sharing private transcripts. A workspace rename or project switch must not retarget an existing envelope/approval.

## Proposed lifecycle

Closing a terminal should detach and leave authorized work running under its original deadline, grants and budget. A second local terminal reconnects after ownership checks, shows missed approvals/conflicts and continues without automatic privilege renewal. This is a future default UX; current P2 requires explicit detached mode and attached disconnect pauses. No remote UI now. Reboot recovery is configurable and begins with worker/owner/grant/budget/artifact reconciliation; do not equate restartability with continuous execution while powered off.

Safe Quit is confirmed with explicit scope (session/project/all), stops new admissions, notifies Solo/Lead to checkpoint promptly, drains bounded safe work and preserves both completed and incomplete records. A finite deadline ends with honest unknown/unreaped failures. Pause/Interrupt is resumable, retaining unresolved work. Force Stop/Cancel requires confirmation, terminates exactly owned workers and preserves recovery evidence. Revocation bypasses ordinary queues immediately; already completed effects cannot be undone by revocation.

Acceptance: simultaneous independent sessions cannot consume each other's approvals/context; detached bounds expire correctly; reconnect does not duplicate workers; Safe Quit preserves incomplete tasks; force stop cannot target unrelated units; shared-root human edits survive; worktree integration/reversal never discards later changes. Current P3 one-project graph ownership is evidence for components, not proof this global/session design exists.
