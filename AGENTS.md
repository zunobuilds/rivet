# AGENTS.md — Rivet

## Product boundary

- Rivet is the independent, open-source AI coding-agent platform. It owns agent modes (Solo, Pair, Crew, Fleet, Mesh, Adaptive), models, Cookbook, tool permissions, coding/build/test/Git workflows, browser automation with Playwright, and qualified desktop application inspection.
- Rivet must work **without** Unrivet; Unrivet is an optional reverse-engineering extension with its own repository. Do not edit or inspect that repository.
- Current release direction: useful pre-1.0 milestones, then Linux-only v1.0.0; macOS afterwards, Windows later. Do not advertise planned features as implemented.

## Strict filesystem boundary

- The **only authorized project directory** is `/home/zuno/Projects/rivet-unrivet/rivet/` (the `rivet` repository root).
- **Never read, open, list, search, inspect, copy, create, edit, move, or delete any files outside this directory unless the user explicitly authorizes that specific outside access.** This includes the parent directory, sibling `../unrivet/`, other repositories, home configuration, mounted drives, and paths reached through symlinks.
- Do not run parent-directory, system-wide, or sibling-repository searches. Keep working directory, generated files, temporary data, dependency caches, and build artifacts inside this repository; if a command cannot comply, ask first.
- Full-access execution permissions do **not** override this rule. Operating-system reads intrinsic to running an authorized executable are not permission to inspect host files.
- The `codex-status` shell command is **explicitly authorized solely to check the user's five-hour allowance**; do not use it as permission to inspect or change other files outside the project. If any other out-of-scope access is needed, ask for explicit approval before proceeding.

## Documentation guide

- `./GITHUB_DOCUMENTATION_GUIDE.md` is a local, project-owned instruction guide. Read and apply it whenever writing or revising the README, Markdown documentation, community files, examples, or repository organization.
- Follow the guide's evidence-first workflow: inspect only this repository, document implemented vs planned behavior accurately, check links/examples, and report unverified claims.
- The guide is advisory within this repository; it never authorizes looking outside the project, choosing a license, adding unrelated files, or changing scope. If it is missing, report that rather than searching another directory for it.

## Work and Git rules

- Follow the latest explicitly user-approved requirements. A planning document or a suggested feature is not implementation approval.
- Before each implementation stage, obtain explicit user authorization for scope and side effects. Finish only that stage; test, save a durable checkpoint, report results, then **STOP** until the next stage is approved. Usage check-ins are not stage approvals.
- Make changes inspectable and reversible; report failed or unrun checks. Do not pretend planned features exist.
- **Local commits:** For user-approved work, ordinary local Git commits are allowed without separate confirmation. Stage only files within the approved task, use clear commit messages, and never amend, reset, rebase, or rewrite history without explicit authorization.
- **Pushes:** NEVER run `git push` unless the user explicitly authorizes that particular push for this repository and task/branch. Being signed into GitHub, having credentials/a remote, approval to commit, or permission given for a previous push does **not** authorize another push. Check the remote and branch before a permitted push; if there is no suitable remote, stop and report it rather than inventing or changing one.
- **Other remote/destructive operations:** Never create a GitHub repository, add/change remotes, force-push, publish a release, rewrite Git history, or perform destructive Git actions without separate explicit user authorization. The repo is locally scoped even when GitHub access is available.
- Do not install software, download models, change host configuration, or initiate paid/network actions unless covered by explicit approval.

## Codex usage checkpoints

At each usage checkpoint, run `codex-status` using the shell. Use its five-hour remaining percentage to determine the check-in interval. Never guess.

- Below 18%: check in every 5 minutes of sustained work.
- 18% or above: check in every 10 minutes of sustained work.
- Save progress and summarize work and findings at each checkpoint. If `codex-status` returns a valid five-hour remaining percentage, continue without asking the user or waiting for a reply.
- If the command fails or does not provide a usable five-hour remaining percentage, ask the user for the current usage and wait for the reply before continuing. Do not infer usage or a reset from elapsed time, stale reports or another allowance.
- Below 10% remaining, slow down: stop starting new batches, save state and wrap up. Below 5%, stop new work and leave a complete checkpoint. Never guess usage; use `codex-status` or the user fallback.
- Shorter runs require no timed check-in. Continue ordinary progress updates independently.
