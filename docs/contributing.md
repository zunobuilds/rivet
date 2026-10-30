# Contributing

Start with the [repository rules](../AGENTS.md), [master plan](../planning/MASTER_IMPLEMENTATION_PLAN.md) and current [verification](../planning/research/post-p3-verification.md). Keep a proposed change bounded and explain its user-visible result, permission effects and evidence. Rivet uses [GPL-3.0-only](../LICENSE). Preserve existing third-party licenses and notices. No separate contributor agreement or copyright assignment has been adopted.

Use the [contained checks](building.md). Add regression coverage for meaningful security/recovery changes. Report ignored native tests, unavailable formatter/Clippy and unrun live checks honestly. Do not weaken isolation, silently acquire dependencies or present synthetic inference as model qualification.

A review should include the problem, implementation, actual test results and remaining limits. Stage only related files. Do not rewrite history or push without the specific authorization required by AGENTS.md. Planned features require implementation scope approval; a roadmap entry is not permission.

Documentation should use original wording, evidence-backed claims and valid relative links. Read the [documentation guide](../GITHUB_DOCUMENTATION_GUIDE.md). There is no established response-time promise or private vulnerability route yet; see [security](security.md).

Before proposing publication, compare public claims to the latest evidence, inspect exact changes for credentials/private paths, check local links/anchors and `git diff --check`, and confirm the authorized branch/remote. Passing compiler/tests does not select a license, authorize a release or qualify unrun platforms. No CI badge or support promise should be added until its target and policy exist.
