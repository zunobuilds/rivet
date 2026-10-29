# Instructions for Codex: GitHub documentation and repository practices

Use this guide when creating or improving a repository's README, Markdown documentation, supporting text files, and file organization. Adapt it to the actual project and the user's request.

Research checked: 2026-10-28. GitHub behavior is linked to official documentation throughout. Other recommendations are editorial defaults, not universal GitHub requirements.

## How to use this file

Give Codex this file with a request such as:

> Read GITHUB_DOCUMENTATION_GUIDE.md and apply it to this repository. Inspect the project first, improve the README and relevant supporting documentation, check links and examples, and report unresolved facts. Create only the files this project needs.

This is a reusable instruction document, not a README template to publish unchanged. Follow applicable repository instructions and the user's requested scope. Do not move application files, change licensing, or introduce tooling merely to make the repository resemble an example.

## 1. Inspect before writing

1. Read applicable repository instructions and existing documentation.
2. Inspect the actual file tree, package manifests, lockfiles, configuration, entry points, examples, tests, and CI workflows.
3. Identify the intended audience, project type, supported environment, installation method, primary use case, and maintenance status from evidence.
4. Read the existing license and notices before describing permissions or attribution.
5. Check which community files and templates already exist, including applicable organization defaults when available.
6. Compare documentation with implementation. Resolve contradictions using evidence; report uncertainty when evidence is incomplete.

Do useful work with available information. Ask focused questions only when a consequential fact cannot be established, such as the owner's license choice or an actual security reporting contact. Continue independent documentation work while that fact is unresolved.

Never invent commands, features, releases, support promises, maintainers, contact addresses, URLs, benchmark results, compatibility, certifications, or deployment status. Separate implemented behavior from proposals. Do not describe a project as production ready or secure without evidence.

## 2. Write for the reader

- Start with what the reader needs to understand or accomplish.
- Use plain language, short paragraphs, descriptive headings, and consistent terminology.
- Explain prerequisites before steps and show the expected result after important commands.
- Use numbered lists for sequences, bullets for parallel points, and tables for comparisons.
- Define uncommon abbreviations on first use.
- Keep one authoritative explanation of a topic and link to it elsewhere.
- Remove empty sections, stale claims, duplicate instructions, and generic marketing language.
- Match the repository's language and naming conventions unless asked to change them.
- Keep essential instructions in text. Give informative images useful alternative text and redact screenshots.
- Use badges sparingly and only for real, relevant resources; verify their targets and claims.

For larger documentation sets, separate learning walkthroughs, task instructions, reference material, and explanations. Create separate pages when readers benefit, rather than forcing every project into four folders. This distinction follows [Diátaxis](https://diataxis.fr/).

## 3. Make the README a useful entry point

A reader should quickly understand the project's purpose, find a first working example, and locate help. Prefer a root `README.md` for a new repository. GitHub recognizes READMEs in `.github/`, the root, and `docs/`, in that precedence order; inspect existing files before adding another. See [GitHub's README documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes).

Choose sections appropriate to the project:

| Section | What to include |
| --- | --- |
| Project name and description | What it does, intended users, and its main use case. |
| Status | Verified limitations, experimental status, or archival status when relevant. |
| Quick start | Prerequisites, minimal setup, first use, and expected result. |
| Usage | Practical examples for the main supported workflows. |
| Configuration | Required settings, defaults, secret handling, and detailed reference link. |
| Documentation | Links to deeper guides, examples, and reference material. |
| Limitations and troubleshooting | Known constraints and actionable fixes for common failures. |
| Contributing and support | Actual contribution process and available help channels. |
| License and acknowledgments | Existing license, relevant credits, and third-party notices. |

Do not make every section mandatory. A small script may need only a description, requirements, usage, and license information. A documentation collection may need navigation instead of installation. A library needs an import or API example; a web application may need environment setup and a local URL.

Keep lengthy operational guides and exhaustive API descriptions in linked documentation. Add a manual contents list only when it helps navigation, and keep its anchors current.

### Quick-start quality

- Derive commands from the repository's real scripts and tooling.
- State required runtime versions, operating system assumptions, and working directory when relevant.
- Use the project's existing package manager and lockfile conventions.
- Distinguish installing a released package from developing a source checkout.
- Provide the shortest complete path to a useful result, including required configuration.
- Identify example output as illustrative if it has not been observed.
- Document cleanup or stopping a local service where needed.
- Flag costs, data modification, elevated permissions, or external account requirements before the affected step.
- Never imply that a documentation change was validated by tests that were not run.

### Adaptable outline

The following is an outline, not completed project documentation. Replace bracketed instructions with verified details and remove irrelevant sections. Include real command blocks after inspecting the project.

```markdown
# [Project name]

[One or two sentences describing the project and its intended use.]

## Quick start

[Prerequisites, verified setup commands, first action, and expected result.]

## Usage

[A realistic example and any important constraints.]

## Documentation

[Links to existing or newly created documentation.]

## Contributing

[Actual contribution instructions or a link to them.]

## License

[Accurate license information and a link to the existing license file.]
```

## 4. Use predictable file organization

Preserve a working structure and the ecosystem's conventions. Documentation work alone does not justify moving source files or renaming directories.

For a new project, this is one possible layout. Every entry beyond the root README is conditional; do not create empty directories or pretend files exist.

```text
project/
├── README.md
├── LICENSE
├── CONTRIBUTING.md
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── CHANGELOG.md
├── .gitignore
├── .env.example
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.yml
│   │   └── feature_request.yml
│   ├── pull_request_template.md
│   └── workflows/
├── docs/
│   ├── README.md
│   ├── installation.md
│   ├── configuration.md
│   ├── troubleshooting.md
│   └── assets/
├── examples/
├── src/
└── tests/
```

- Keep widely recognized filenames such as `README.md` and `CONTRIBUTING.md` consistent. Prefer descriptive lowercase names with hyphens for ordinary documentation pages.
- Separate source, tests, examples, documentation, and generated output when the project warrants it.
- Link important documentation from the README. Add a documentation index when several pages need navigation.
- Show only useful paths in published directory trees and update trees when paths change.
- Keep assets near the documentation that uses them; avoid unnecessary large binaries.
- Do not commit dependency caches, local environments, credentials, or generated output unless the project deliberately tracks them.
- Do not ignore all lockfiles or all generated files automatically; follow ecosystem and project requirements.
- Remember that `.gitignore` does not stop tracking an already tracked file or remove secrets from history. See [GitHub's ignore-file guidance](https://docs.github.com/en/get-started/git-basics/ignoring-files).

## 5. Choose supporting files deliberately

| File | Add or update when | Required substance |
| --- | --- | --- |
| `LICENSE` | The owner has selected a license, or an existing license needs an accurate link. | Complete authorized license text and correct required details. |
| `CONTRIBUTING.md` | The project accepts contributions. | Working setup, checks, contribution workflow, and review expectations. |
| `SECURITY.md` | Users need vulnerability reporting guidance. | Supported versions if established and a real private reporting route. |
| `CODE_OF_CONDUCT.md` | A community needs defined participation standards. | Adopted standards, enforcement process, and a monitored contact. |
| `CHANGELOG.md` | A maintained change history helps users. | Actual changes, compatibility impact, and verified release information. |
| `SUPPORT.md` | Help routes need more detail than the README. | Existing support channels and boundaries. |
| `CITATION.cff` | Research or other work needs structured citation metadata. | Verified authors, title, identifiers, and version details. |
| `DISCLAIMER.md` | Substantial project-specific caveats need a dedicated page. | Applicable limitations, linked from relevant documentation. |
| `.env.example` | The application uses environment configuration. | Real variable names and safe example values, never working secrets. |
| Third-party notices | Included materials have applicable attribution requirements. | Accurate notices consistent with the relevant licenses. |

GitHub supports community files in specific locations and can inherit supported defaults from an account's public `.github` repository. Inspect those defaults before duplicating policy. A project license must accompany the individual repository. See [GitHub's community health file documentation](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/creating-a-default-community-health-file).

A security policy documents a reporting process; it does not enable private vulnerability reporting. Confirm that the proposed route works before directing users to it. Do not send sensitive vulnerability details to public issues. See [GitHub's security policy guidance](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/add-security-policy).

Avoid invented response deadlines, support periods, moderation teams, or donation links. If owner input is necessary, report the missing decision instead of publishing a placeholder policy as final.

### Issue and pull request templates

Use `.github/ISSUE_TEMPLATE/` for issue templates or YAML issue forms and their optional `config.yml`. Markdown issue templates need appropriate YAML frontmatter; issue forms must follow GitHub's schema. Templates become available through the repository's default branch. See [issue template configuration](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository).

Ask bug reporters for reproduction steps, expected and actual behavior, relevant versions, and sanitized logs. Ask feature requesters for the problem and intended outcome. Avoid unnecessary required fields and do not request credentials or personal data.

A single PR template can live at `.github/pull_request_template.md`. Ask for the problem, resulting behavior, relevant issue, validation, and material limitations. Use `Closes #123` only when the PR resolves that issue. Keep checklists relevant and unchecked until verified. See [GitHub's PR template documentation](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository).

## 6. Format Markdown consistently

These are house-style defaults; preserve compatible existing conventions:

- Use one `#` title per standalone document, followed by logical `##` and `###` levels.
- Put blank lines around headings, lists, tables, and fenced code blocks.
- Use backticks for filenames, identifiers, flags, and commands.
- Use fenced code blocks with suitable language labels, such as `bash`, `powershell`, `json`, `yaml`, or `text`.
- Keep command output separate from commands. Avoid shell prompts that readers may accidentally paste.
- Preserve valid indentation in YAML, nested lists, and code examples.
- Keep lines readable in source and follow the repository's formatter; avoid arbitrary manual wrapping inside URLs or commands.
- Prefer relative links for repository files and assets. Check exact filename case and resolve paths from the document containing the link.
- Use descriptive link labels and verify section anchors after changing headings.
- Use simple tables for short comparisons; use prose or lists for complex explanations.
- Prefer standard Markdown. Use HTML, collapsible details, Mermaid, or GitHub alerts only when they improve understanding and suit the target renderer.
- Keep essential instructions visible outside collapsed sections. GitHub-specific formatting may need a portable fallback for other renderers.
- End text files with a newline and avoid unnecessary trailing whitespace.

See GitHub's [formatting syntax](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax) and [code-block guidance](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/creating-and-highlighting-code-blocks).

To show Markdown containing triple-backtick blocks, surround the example with a longer fence:

````markdown
```bash
git status --short
```
````

## 7. Handle licenses, disclaimers, and attribution accurately

Licenses establish permissions and conditions; disclaimers describe limitations or cautions. A disclaimer is not a substitute for a license. Public visibility alone does not make a repository open source. Preserve existing license text and do not choose or change a license without owner direction. See [GitHub's licensing guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository).

Add a disclaimer only for an actual project concern. Use a short README section for a simple limitation or link a longer document when needed. Put operational warnings immediately before the affected step as well.

Possible cases:

| Concern | Appropriate documentation |
| --- | --- |
| Experimental implementation | Identify unstable behavior and known unsupported uses. |
| Destructive operation | Identify affected data, prerequisites, backup needs, and recovery limitations. |
| External services | Explain actual account, cost, availability, and data-sharing dependencies. |
| Unofficial integration | State verified independence and avoid implying endorsement. |
| Sensitive professional domain | Explain the tool's actual scope and limitations without promising legal protection. |
| Third-party content | Credit sources and preserve required license notices. |

Example wording, usable only when true:

> This project is experimental. Configuration formats may change between releases. Back up existing configuration before upgrading.

> This tool sends the submitted text to the configured external API. Review the provider's data handling terms before submitting confidential information.

Avoid blanket disclaimers pasted into every project, unsupported claims of anonymity or compliance, and invented warranty language. Link to the actual license for its terms. Never claim that a disclaimer guarantees immunity from liability. Flag consequential legal wording for owner review rather than presenting it as settled legal advice.

## 8. Keep examples safe and reproducible

- Use synthetic data and clearly fake credentials. Use reserved domains such as `example.com` for illustrative addresses.
- Mark placeholders explicitly in templates; replace them before publishing project-specific instructions.
- Keep secrets out of commands, URLs, screenshots, logs, and sample configuration.
- Describe what environment variables do and whether they are required, optional, sensitive, or externally transmitted.
- Explain elevated permissions and destructive operations before commands. Prefer a dry run when the tool supports it.
- Do not execute risky example commands merely to validate prose. Inspect them and report validation limits.
- Keep examples small but complete enough to reproduce the stated result.

## 9. Validate before finishing

Review the result against the actual repository:

- [ ] Every documented feature, path, script, option, and support claim has evidence.
- [ ] The quick start has all prerequisites and a clear success condition.
- [ ] Safe commands were checked where feasible; unexecuted commands are identified in the handoff.
- [ ] Local links, image paths, filename case, and heading anchors resolve.
- [ ] External links target the intended resources; unavailable checks are reported accurately.
- [ ] Markdown fences, heading hierarchy, tables, and list indentation are valid.
- [ ] Existing Markdown linting, link checks, or documentation builds pass where applicable.
- [ ] Rendering was previewed when available, or the lack of a preview is disclosed.
- [ ] Supporting files contain real policies and contacts, with no accidental placeholders.
- [ ] No secrets, personal information, fabricated badges, or unsupported legal claims were introduced.
- [ ] Documentation agrees with implementation, license, examples, and directory trees.
- [ ] Changes remain within scope and do not overwrite unrelated user work.

Use existing tooling first. Do not add a documentation framework, dependencies, or CI workflow solely to check a small Markdown edit. Avoid tests that only repeat the text being written.

For a repository with Git metadata, inspect the final diff and use `git diff --check` for whitespace problems. It does not validate Markdown or links. If Git metadata is absent, inspect the files directly and do not claim a Git check passed.

## 10. Deliver a complete, honest handoff

Create the requested files rather than stopping at suggestions. Briefly report:

1. Files created or changed and the reader benefit.
2. Checks actually performed and their results.
3. Material uncertainties or decisions requiring owner input.

Do not claim publishing, deployment, enabled GitHub features, or passing tests without evidence. Keep unresolved facts visible instead of silently filling gaps.

## Research references

The linked GitHub documentation establishes platform behavior. The guide's writing workflow, layout example, and quality checklist are a practical synthesis, intended to be adapted rather than treated as mandatory platform rules.

- [GitHub: About READMEs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
- [GitHub: Basic writing and formatting syntax](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax)
- [GitHub: Creating and highlighting code blocks](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/creating-and-highlighting-code-blocks)
- [GitHub: Creating default community health files](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/creating-a-default-community-health-file)
- [GitHub: Configuring issue templates](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository)
- [GitHub: Creating a pull request template](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository)
- [GitHub: Adding a security policy](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/add-security-policy)
- [GitHub: Licensing a repository](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)
- [GitHub: Ignoring files](https://docs.github.com/en/get-started/git-basics/ignoring-files)
- [Diátaxis: Technical documentation framework](https://diataxis.fr/)
