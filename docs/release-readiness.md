# v0.1.0-alpha.1 release readiness

**Status: prepared locally; not ready to publish.** This task authorizes a draft pre-release only. No tag, push, publication, binary upload, tool installation or native/live-provider exception is authorized.

## Version and source identity

| Surface | Observed | Required before publishing |
| --- | --- | --- |
| Cargo.toml | `0.1.0` | `0.1.0-alpha.1` |
| Cargo.lock root `rivet` package | `0.1.0` | `0.1.0-alpha.1`, dependencies otherwise unchanged |
| CLI `--version` | Defined from `CARGO_PKG_VERSION` plus build digest | Rebuild and verify the alpha version and intended build digest |
| Intended release title/tag | `Rivet v0.1.0-alpha.1` / `v0.1.0-alpha.1` | Match the tested version-aligned source commit |

Cargo's current values agree with each other, but not the planned pre-release. They were deliberately left unchanged: this preparation documents the required alignment, rather than presenting an unqualified build as the alpha. An existing executable can also be stale; its output cannot prove the current manifest was rebuilt.

## Readiness checklist

- [x] Adopt [SemVer 2.0.0 policy](versioning.md) and prepare [accurate release notes](releases/v0.1.0-alpha.1.md).
- [x] Inspect roadmap, source entry points and stage verification; distinguish implemented, synthetic, native and future work.
- [x] Confirm intended repository `zunobuilds/rivet`, branch main, and no existing local tags. GitHub read checks returned no releases and no tag matching `v0.1.0-alpha.1`; draft visibility is limited by the connection's permissions.
- [x] Run contained locked offline tests: 77 passed, zero failed, eight ignored. This validates the current development version, not a final alpha commit.
- [x] Confirm locked offline Cargo metadata, license and current version consistency.
- [ ] **Blocker:** align manifest/lock to the alpha version; rebuild and verify CLI version on the final candidate commit.
- [ ] **Blocker:** obtain narrow native-test authority and rerun all eight native tests plus worker/resource probes with cleanup evidence. Prior host passes are historical.
- [ ] **Live qualification gap:** approve an existing endpoint/model, context/data scope and cost limits for an end-to-end live smoke check. Without it, do not claim a live-qualified coding release; any source-only experimental publication needs explicit owner acceptance of that omission.
- [ ] Qualify formatter/Clippy and dependency/advisory checks, or obtain an explicit documented disposition for unavailable checks. No installation is implied.
- [ ] Complete notices/corresponding-source and secret review for exactly the selected source/artifacts; verify backups/migration guidance and supported environment. Maintainer contact is not a verified private security reporting route.
- [ ] After alignment, repeat relevant offline/native checks, bind results and notes to the exact final SHA, and confirm all advertised capabilities have evidence.
- [ ] Create or verify a GitHub draft with `draft=true`, `prerelease=true`, no assets and no latest-release designation, without creating a tag.
- [ ] Obtain separate explicit authorization for source push, exact tag creation/push and eventual publication. This task authorizes none of them.

Native qualification is required to close the current P1–P3 gate. Live qualification is separately untested; synthetic success is not a substitute. Multi-day endurance is optional P7 work under the existing qualification contract, not a newly imposed alpha delay. Never waive a known unsafe effect, credential disclosure or permission bypass as an “alpha limitation.”

## Draft attempt and tooling

The GitHub connector supports read-only release access but exposes no release-creation operation. The installed `/usr/bin/gh` is available; with `GH_CONFIG_DIR` restricted to `.build/release-gh-config`, update checks/prompts disabled, `gh auth status --hostname github.com` returned exit 1: not logged into any GitHub hosts. No host configuration/keychain was inspected and no login or installation was attempted.

**Draft URL: unavailable — no draft was created.** Connector read access does not supply a credential for CLI writes. No release POST/PATCH, tag mutation, push or asset upload was attempted. Do not infer that there are no private drafts merely from the visible empty release list.

GitHub's [release API](https://docs.github.com/en/rest/releases/releases#create-a-release) supports draft/pre-release flags. Its [CLI documentation](https://cli.github.com/manual/gh_release_create) warns that missing tags can be created automatically. Therefore do not run a generic release-create command in this task. A later authenticated path must explicitly preserve draft state and prove it creates no Git ref; if that guarantee is unavailable, save the draft locally as done here.

## Exact next steps

1. Review these notes and choose the final alpha scope; authorize the version-alignment change and missing qualification access.
2. Set Cargo.toml to `0.1.0-alpha.1`, regenerate only the root lock entry using the contained offline route, and inspect the diff for unintended dependency changes.
3. Rebuild and run the relevant offline/native checks plus the separately approved live smoke test or explicit source-only disposition. Record final SHA, environment, failures and supported scope; do not reuse stale binary/test claims.
4. Provide a permitted authenticated release-writing route that keeps all local configuration within the repository. Check for an existing draft to avoid duplicates, then save only a draft/pre-release with the reviewed notes and no assets or new tag. Replace this status with its returned URL/ID only after verified success.
5. Retarget and verify any draft after the final tested source is pushed under separate authorization. Keep publication disabled until all blockers and the specific tag/publication permissions are resolved.

## Preparation validation record

Baseline: `ad92aea18bbf3614f32448be81a1a9e40a5a2473`. No existing release procedure or release workflow was found in tracked files; `.github/` is absent. This document establishes the focused preparation handoff, not an automated publisher.

Current run logs remain repository-local under `.build/release-metadata.json` and `.build/alpha1-release-tests.txt`. External research used the SemVer specification and GitHub's official release API/CLI documentation; alpha/beta/rc stage criteria are Rivet conventions. No changes to application code, package versions, dependencies, AGENTS.md or historical evidence are included.


Final preparation checks passed for the five changed/new Markdown documents: 39 local links/anchors, one title per file, final newlines, common secret-pattern review and `git diff --check`. Source review confirmed CLI version comes from Cargo metadata. No rendered Markdown preview or final version-aligned/native/live-model qualification is claimed. The local commit contains only release policy, notes, readiness and navigation changes; no package version was changed. No push was performed.
