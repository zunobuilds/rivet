# Agent execution proposal

**P0.6 authority note:** The [master implementation plan](MASTER_IMPLEMENTATION_PLAN.md) selects the current stack, defaults and contracts; [decision dispositions](open-decisions.md) distinguish owner approvals from technical recommendations. P0.5 comparisons and proposal wording below are retained as research provenance, not competing current defaults. No implementation is authorized.

**Status:** P0.5 design research only. Exact compositions below are approved requirements from the [brief](RIVET_PROJECT_BRIEF.md); ownership, scheduling and budget mechanics are proposals. Component boundaries live in [architecture](architecture.md), recovery rules in [security and recovery](security-and-recovery.md), and unresolved choices in [open decisions](open-decisions.md).

## Approved mode invariants

| Mode | Exact composition rule |
| --- | --- |
| Solo | Exactly 1 Independent; only mode in Minimal. |
| Pair | Exactly 1 Lead + exactly 1 Operator. |
| Crew | Exactly 1 Lead + exactly 1 Reviewer + Operators >= 1. |
| Fleet | Exactly 1 Director + Leads >= 1 + Reviewers >= 1 + Operators >= 1. |
| Mesh | Configurable peers; precise ownership semantics unresolved. |
| Adaptive | Select among valid modes without violating their count rules. |

Role identity, loaded model identity and concurrently executing request count are distinct. Creating multiple model clients does not create valid roles, and sharing weights does not merge agents' authority or context. Validate a complete mode configuration before starting and after any permitted transition.

## Proposed responsibilities and ownership

Independent owns its Solo plan, execution and review evidence. Lead owns decomposition and integration within its assigned work; Operator executes scoped tasks; Reviewer evaluates specific revisions/evidence; Director owns Fleet-wide dependencies and resource allocation. These assignments do not elevate tool permissions. Reviewer approval is technical evidence, not human permission for an external effect.

Each executable task should have one accountable owner at a time, plus explicit parent/dependencies and completion criteria. Reviewer feedback can create a new revision request, but should not mutate a running Operator's task in place. A parent cannot complete while required children, reviews or unknown operations remain unresolved.

Use a task dependency graph with cycle detection. Parent completion references tested artifact/revision IDs rather than conversational assertions. Prevent circular “waiting on reviewer/lead” deadlocks through explicit wait reasons and supervisor diagnostics. A failed reviewer does not silently count as a passed review.

For shared workspaces, recommend one writer lease per overlapping path set and a serialized Git index/ref mutation lane. Read-only tasks may run concurrently against recorded revisions. Worktrees are an alternative for independent branches, with explicit merge/integration cost; worktrees do not isolate secrets, hooks, network or host processes. Human edits always invalidate stale write preconditions. Lease expiry alone cannot authorize duplicate dispatch: fencing tokens and old-process reconciliation are required.

## Scheduling

Recommend a single supervisor-managed ready queue with bounded worker slots, provider/model semaphores and ledger-owned assignments. Eligibility requires valid hierarchy, satisfied dependencies, current grants, resource reservation and workspace preconditions. Fair scheduling/aging prevents one chatty task from starving review or cancellation. Priority policy should be visible and explainable, not a second background optimizer.

Proposed task states: pending, ready, assigned, running, waiting-input, waiting-resource, waiting-review, reconciling, succeeded, failed, cancelled. Only the supervisor commits state transitions; worker outputs are observations. Durable timers include deadlines/retry eligibility, while in-memory wakeups are reconstructible hints. Recompute elapsed deadlines after restart; wall-clock adjustments must not silently extend an authorization window.

Pause means stop new dispatch and bring active work to a documented safe point; stop means cancel and reconcile. Revoked permission, exhausted budget, unavailable provider or unsafe replay can require human input. Multi-day authorization never means “keep guessing when blocked.”

## Specialists and mode changes

A specialist is a temporary skill/task assignment within an existing permitted role count, not a loophole to spawn extra agents. Examples: Pair's Operator can perform a Rust-focused subtask, or Crew can allocate an additional Operator while keeping one Lead and one Reviewer. Solo cannot temporarily create a Reviewer while claiming to remain Solo.

Proposed mode transition: request and approve change under the user's permitted policy; quiesce affected work; reconcile active operations; atomically replace the validated configuration; increment routing/ownership epoch; reassign pending work and inform the user. Adaptive may choose only permitted valid configurations inside an explicit budget. If a transition is not preauthorized, ask rather than silently changing modes.

Mesh proposal: one owner per task even among peers; deterministic claim/fencing and a designated terminal decision route. Peer equality does not require multi-writer tasks or leader-election infrastructure. Mesh admission, conflict arbitration and peer escalation remain D07; defer execution until they have testable semantics.

## Messaging and context

Persist messages in the same ledger with sender, recipient, task, causal/reference IDs, sequence, delivery state and acknowledgment. Recommend at-least-once delivery with idempotent consumption; acknowledgment and resulting assignment transition should commit together where possible. Duplicate delivery must not repeat a patch or provider call. Bound queues and detect unanswered dependency messages.

Route human instructions through the main terminal chat. Preserve the approved consent requirement for forwarding important user decisions: record who authorized which decision to reach which recipients, and carry the source decision reference. Do not let agents paraphrase a user decision into broader permission. Whether routine progress reports need individual forwarding consent is unresolved; propose a scoped routing policy approved at run start.

Context should be assembled from role instructions, task dependencies, trusted user decisions and selected evidence. Mark file/web/browser/tool/peer content with provenance and trust class. Tool output and human file edits are data, not elevated instructions. Include relevant constraints across every compaction; a summary must not replace the broker's policy state.

Preserve source references, artifact hashes, tested revision IDs and model/config identities alongside summaries. Count context against provider limits; bound per-file/tool output, reserve response budget, and request clarification when truncation hides essential evidence. Do not copy a full global transcript into every agent. Cross-project memory and vector databases are deferred unless a concrete need and privacy policy justify them.

## Concurrent, Shared Model and Model Relay

These approved configurable behaviors are independent axes of mode selection; their exact runtime contracts remain proposals.

| Behavior | Proposed execution rule | Resource/quality caveat |
| --- | --- | --- |
| Concurrent | Run eligible agents simultaneously within endpoint and host limits | Multiple tool calls can conflict even when inference capacity is available. |
| Shared Model | Route distinct role contexts to one permitted model instance/provider | Shared weights do not eliminate per-request key/value (KV) cache and context memory or queueing. Never mix role histories accidentally. |
| Model Relay | Serialize eligible role/model phases, hand off durable evidence, explicitly load/unload only if authorized | Loading, warmup and context replay can cost time/memory; no assumed speed advantage. |

llama.cpp exposes server slots and continuous batching, but model/templates constrain tool calling. This supports a capacity-aware adapter, not a universal concurrency promise. [Server documentation, S23](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)

Relay should refer to evidence/artifacts and bounded summaries rather than assume hidden-state transfer between unrelated providers. A provider failure must not change local/cloud placement or send new project data without permitted policy. Retain unknown billing state after interrupted requests.

## Budgets and Cookbook

Propose budgets at run and task levels: wall-clock deadline, inference token/currency ceiling, remote request rate, concurrency, RAM/VRAM estimates, context size, process count, tool time, output/artifact disk limits and retry ceiling. Shared parent budgets prevent children from multiplying spending authority. Reserve estimated request/output capacity before dispatch, reconcile actual usage after completion and release reservations only when safe.

Provider-reported usage may be delayed or incomplete; maintain an uncertainty reserve, stop when a conservative bound would exceed the approved ceiling, and label estimates. A local token budget is not a guaranteed external billing cap; provider-side account limits are complementary. Hardware estimates should include shared weights once per actual resident instance, per-request cache, browser/tool memory, OS headroom and peak relay/loading overlap.

Cookbook should reuse qualified `llmfit` JSON/fit data, then evaluate the entire hierarchy and explain serial queues, unsupported models and memory uncertainty. `llmfit` also exposes benchmarking and other workflows: do not invoke them merely to obtain recommendations. [llmfit, S26](https://github.com/AlexsJones/llmfit)

Efficient/Balanced/Ambitious are approved priorities, not fixed numerical performance promises. Propose transparent scoring over fit, latency estimate, cost and user preference; expose the reason for each allocation. Automatic optimization beyond this remains deferred. Benchmarking requires separate explicit authorization; P0.5 ran none.

## Alternatives and acceptance evidence

Using LangGraph's checkpoints as a reference may reduce conceptual design work, but embedding it would introduce Python/JS orchestration beside the Rust supervisor. Rig's narrower provider abstraction may offer reuse without transferring scheduling authority. Evaluate both at an approved implementation gate; neither framework proves Rivet's recovery/permission invariants. [LangGraph, S18](https://docs.langchain.com/oss/python/langgraph/persistence), [Rig, S17](https://github.com/0xPlaygrounds/rig)

Future tests should cover every valid/invalid composition, specialist invariants, permitted/refused transitions, duplicate/out-of-order messages, ownership fencing, two writers racing a human edit, dependency cycles, review failure, cancellation under saturation and aggregate budget exhaustion. Use deterministic fake providers first; actual model benchmarks remain opt-in. The [bounded qualification contract](P2_P3_QUALIFICATION.md) sets required P2/P3 stress/recovery/cancellation gates before testing. Optional multi-day endurance moves to P7 and requires its own predeclared scope/duration; it is not a P2/P3 dependency.
