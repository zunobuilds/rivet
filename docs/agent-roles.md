# Agent roles and modes

Rivet separates **roles** from **modes**. A role describes an agent's responsibility; a mode defines how agents are arranged. **Solo is a mode whose single agent has the Independent role.** Lead, Operator, Reviewer and Director are roles used in larger compositions.

This guide describes the current bounded implementation. Agents work within explicitly reviewed task graphs, permissions and budgets. Synthetic tests demonstrate the documented flows; live model quality and unrestricted autonomous delegation are not established. See [modes and model policies](modes.md) for execution options and [verification](../planning/research/post-p3-verification.md) for evidence and limits.

## Roles at a glance

| Role | Main responsibility | Used in |
| --- | --- | --- |
| Independent | Carry a task through implementation and checks without a reporting hierarchy | Solo and Mesh |
| Lead | Coordinate a group and integrate its required results | Pair, Crew and Fleet |
| Operator | Perform assigned implementation work and provide evidence | Pair, Crew and Fleet |
| Reviewer | Evaluate implementation against its recorded revisions and checks | Crew and Fleet |
| Director | Coordinate the overall run above its Leads | Fleet |

## Independent: the Solo agent or Mesh peer

An Independent agent handles its assigned coding work directly: inspect relevant files, propose changes, request checks and report results through authorized tools. In Solo, the same agent handles the task without a separate Lead, Operator or Reviewer. There is no dedicated second-agent technical review.

In Mesh, several Independent agents work as peers. Each task still has one accountable owner; peers do not all gain permission to edit the same files. A designated root peer integrates results and routes conflicts to the main chat without becoming a Lead or gaining extra permissions.

Example: a Solo agent fixes a small parser bug, requests the relevant test and reports the resulting diff and check evidence.

## Lead: group coordination

A Lead coordinates the group's assigned scope and integrates child-task results. Operators and Reviewers report to a Lead. In Pair and Crew, the Lead is the top-level coordinator; in Fleet, each Lead reports to the Director.

In the current task graph, coordination tasks declare no write resources. Coding work belongs to Operators, while the Lead consumes progress and evidence. A coordinator's completion text cannot make required failed or unresolved child work successful.

Example: a Crew Lead coordinates an Operator's parser fix and a Reviewer's assessment, then integrates their recorded results.

## Operator: implementation work

An Operator performs an assigned coding task under a Lead. It can propose file changes, checks and supported Git operations within the task's declared scope, subject to the normal permission broker.

The Operator supplies recorded revisions and check evidence when the task requires checked completion. A statement such as “tests passed” does not replace an actual check receipt. In Crew and Fleet, required coding tasks must have required technical review.

Example: an Operator changes the selected parser files and runs a check against the exact source projection recorded in the result.

## Reviewer: independent technical assessment

A Reviewer examines the assigned coding task's exact recorded revisions and checks, reads relevant source through reviewed tools, and returns a structured technical decision: approve or request changes.

Review tasks declare no write resources. The Reviewer evaluates the change rather than silently repairing it. Missing, failed or stale evidence prevents successful required review; approval must refer to the actual subject evidence.

**Technical approval is not user permission.** A Reviewer cannot authorize a tool operation, expand file access or approve another agent's privilege escalation.

Example: after an Operator's check succeeds, the Reviewer inspects the changed code and its evidence. If a later observed edit makes that evidence stale, the earlier approval cannot establish that the new revision was checked.

## Director: overall coordination

A Director is the top-level coordinator in Fleet. Leads report to it; Operators and Reviewers report to their respective Leads. The Director integrates the required group results rather than replacing each group's implementation and technical review.

Like a Lead, a Director's coordination tasks do not declare write resources. Its position in the hierarchy grants no additional filesystem access, spending authority or ability to approve tools.

Example: a reviewed Fleet plan assigns separate groups to two parts of a change. Their Leads integrate local implementation and review results, then the Director brings the required results together.

## How the modes combine roles

| Mode | Required composition |
| --- | --- |
| Solo | Exactly one Independent |
| Pair | Exactly one Lead and one Operator |
| Crew | Exactly one Lead, exactly one Reviewer and at least one Operator |
| Fleet | Exactly one Director, at least one Lead, at least one Reviewer and at least one Operator |
| Mesh | Configurable Independent peers; the current template accepts 1–32 |
| Adaptive | An explicit policy selecting permitted concrete compositions at safe boundaries |

Adaptive is not another agent role. It can change the composition only within its reviewed policy, count ceilings, remaining budgets and transition rules. It cannot silently remove required review or create unrestricted agents. Minimal permits Solo only.

For a simple comparison, Pair separates coordination from implementation. Crew adds a dedicated technical Reviewer. Fleet adds a Director above the Leads. Mesh uses peers instead of reporting relationships. These shapes do not guarantee faster or better results; suitability depends on the task, model and available capacity.

## Specialists, models and permissions

A specialist is an assignment of expertise or configured skills, not a sixth core role. Specialist assignments must fit the active mode's role counts and approved task scope. They do not create extra authority. Future mod-provided specialist roles remain [research](../planning/research/post-p3-product-ui.md).

Agent identity is also separate from model identity. Several agents may use the same full model weights while keeping separate task contexts and context-cache budgets, or use dedicated model instances. Sharing a model does not merge conversations or permissions.

All roles use the same trusted supervisor, broker, registry and ledger. Role names and peer messages cannot grant tool access. Every operation remains subject to current grants, ownership, declared resources and run limits. See [security and permissions](security.md).
