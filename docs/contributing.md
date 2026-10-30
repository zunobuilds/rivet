# Contributing

Contributions should make Rivet more useful while preserving its permission, isolation and recovery boundaries. Keep changes focused and support behavior claims with evidence.

## Before you start

Read the [repository rules](../AGENTS.md), [master plan](../planning/MASTER_IMPLEMENTATION_PLAN.md) and latest [verification](../planning/research/post-p3-verification.md). Planned features require explicit implementation approval; a roadmap entry alone does not authorize work.

Use the [build guide](building.md) to set up and run the existing contained checks. Do not install tools, acquire models or widen permissions without the authorization required by the repository rules.

## Proposing changes

Explain the problem, expected behavior and scope before substantial implementation. Keep unrelated refactoring separate so reviewers can assess each change.

For bug reports, include reproduction steps, expected and actual results, the relevant version or build digest, and sanitized diagnostics. Remove credentials and private project content.

For a change submitted for review, include:

- The problem and resulting behavior.
- Any changes to permissions, data handling or recovery.
- Checks performed, their results and any checks left unrun.
- Known limitations or compatibility concerns.

Stage only related files. Do not rewrite history or push without the specific authorization required by AGENTS.md.

## Validation and documentation

Run the relevant [contained checks](building.md) and add regression coverage for meaningful behavior, security or recovery changes. Report ignored native tests, unavailable tools and unrun live checks explicitly. Synthetic provider tests do not establish live model compatibility.

Use original wording, evidence-backed claims and working relative links in documentation. Follow the [documentation guide](../GITHUB_DOCUMENTATION_GUIDE.md), distinguish implemented behavior from plans, and run `git diff --check` before submitting changes.

Before publication, inspect the diff for secrets and private data, compare public claims with the latest evidence, and confirm the authorized branch and remote. Passing tests does not authorize a release or qualify untested platforms. Add badges and support promises only when their targets and policies exist.

## License

Rivet uses [GPL-3.0-only](../LICENSE). Preserve existing third-party licenses and notices. No separate contributor agreement or copyright assignment has been adopted.

## Maintainer contact

For contribution questions, contact the project maintainer at [zuno.githubcontact@gmail.com](mailto:zuno.githubcontact@gmail.com).

See [security and permissions](security.md) for current security reporting limitations. Do not include credentials or sensitive vulnerability details in public reports. No response-time commitment is currently established.
