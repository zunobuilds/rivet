# Agent roles and modes

A **role** defines an agent's responsibility. A **mode** defines how agents work together. Solo is a mode with one Independent agent; Lead, Operator, Reviewer and Director are roles within larger teams.

## Roles

### Independent — handles the task directly

Inspects files, implements changes, runs authorized checks and reports results. In Solo, one Independent handles the whole task. In Mesh, Independents work as peers with separate task ownership; a designated root peer integrates results and brings conflicts to the main chat.

### Lead — coordinates a group

Coordinates assigned work and combines the results from Operators and Reviewers. The Lead heads Pair and Crew; in Fleet, it reports to a Director. Current coordination tasks do not write project files, and required unresolved work blocks successful completion.

### Operator — implements changes

Performs assigned coding work under a Lead: edits files, requests checks and proposes supported Git operations within its scope. It provides revision and check evidence. In Crew and Fleet, required coding tasks also require technical review.

### Reviewer — assesses the implementation

Examines the assigned changes and their exact recorded revisions and checks, then approves or requests changes. Review tasks do not write project files. Missing or stale evidence prevents successful required review. Technical approval does not grant tool permissions.

### Director — coordinates the overall run

Heads Fleet and integrates results across its Leads. Operators and Reviewers report to their respective Leads. Like a Lead, the Director coordinates rather than directly modifying project files; its position grants no additional permissions.

For example, in Crew an Operator fixes a parser bug and supplies test evidence, a Reviewer assesses the change, and the Lead integrates the required results.

## Modes

| Mode | Composition |
| --- | --- |
| Solo | Exactly one Independent |
| Pair | Exactly one Lead and one Operator |
| Crew | Exactly one Lead, exactly one Reviewer and at least one Operator |
| Fleet | Exactly one Director, at least one Lead, at least one Reviewer and at least one Operator |
| Mesh | Independent peers; the current template accepts 1–32 |
| Adaptive | Selects permitted compositions at safe transition points |

Adaptive is a mode-selection policy, not another role. Changes must preserve required review, role counts, permissions and budgets. Minimal permits Solo only. See [modes and model policies](modes.md) for execution options.

## Shared rules

All agents work within explicitly reviewed task graphs. Roles describe responsibility; the trusted supervisor and permission broker enforce access and run limits. No agent can approve its own privilege escalation.

A specialist adds expertise or configured skills within an existing role and scope. It does not bypass the mode's count rules or gain extra authority. Mod-provided specialists remain [planned](../planning/research/post-p3-product-ui.md).

Several agents can share full model weights while retaining separate contexts and budgets, or use dedicated model instances. Sharing a model does not merge conversations or permissions.

The current flows have bounded synthetic-test evidence, not live-model quality or performance guarantees. See [verification](../planning/research/post-p3-verification.md) and [security](security.md) for limits.
