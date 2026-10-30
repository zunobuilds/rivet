# v0.1.0-alpha.1 release status and follow-up

**Status: published pre-release, with outstanding qualification.** The owner published [Rivet v0.1.0-alpha.1](https://github.com/zunobuilds/rivet/releases/tag/v0.1.0-alpha.1) on 30 October 2026 at 15:01:27 UTC. GitHub reports `draft=false`, `prerelease=true` and no uploaded assets. GitHub's automatic source archives are separate from uploaded binaries.

## Alpha.2 preparation

The owner authorized a new package version after the alpha.1 correction. Commit `f8a56d1` sets Cargo.toml and the root Cargo.lock entry to `0.1.0-alpha.2`; Cargo metadata and the dependency-preserving lockfile comparison passed. [Alpha.2 release notes](releases/v0.1.0-alpha.2.md) are prepared locally. An alpha.2 build, CLI-version check and full suite are still pending; the alpha.1 correction results below must not be relabeled as alpha.2 qualification. No alpha.2 tag or GitHub release was created by this preparation.

## Version discrepancy

The published tag points to commit `813db12e72972b7ea275077d61361d15f670acb9`. Its Cargo.toml and root Cargo.lock entry contain `0.1.0`, despite the tag name `v0.1.0-alpha.1`.

The owner subsequently authorized aligning the working source's manifest and lockfile to `0.1.0-alpha.1`. That correction is a new local commit; it does not repair the existing release archive. The dependency-audit script now reads the package version from Cargo.toml instead of hard-coding `0.1.0`.

| Surface | Published alpha.1 tag | Alpha.1 correction (historical) |
| --- | --- | --- |
| Cargo.toml | `0.1.0` | `0.1.0-alpha.1` |
| Cargo.lock root `rivet` entry | `0.1.0` | `0.1.0-alpha.1`; dependency versions unchanged |
| Rebuilt CLI | Derived from the tagged package version | Must report `0.1.0-alpha.1` plus its new build digest |
| GitHub release | Published pre-release at the existing tag | No release, tag or asset changes made by this correction |

Under the [SemVer policy](versioning.md), keep the published tag immutable. Any corrected release must use a new version, such as `0.1.0-alpha.2`, on a separately authorized and qualified source commit. Do not move the old tag, replace its archives or claim that a local version change fixed the published package.

## Follow-up checklist

- [x] Confirm the public release status, tag commit, notes and absence of uploaded assets through read-only GitHub API requests.
- [x] Retain accurate release notes and the official SemVer 2.0.0 policy.
- [x] Align the working manifest and root lock entry to the alpha identifier; preserve all dependency versions.
- [x] Verify the rebuilt CLI, offline suite, dependency audit and documentation checks for the local correction; see results below.
- [ ] Obtain narrow host authority and rerun the eight native tests plus worker/resource probes with cleanup evidence. Earlier native passes remain historical.
- [ ] Separately authorize an existing endpoint/model, data scope and cost limits for a live smoke check. Without it, retain the explicit lack of live-model qualification; synthetic success is not a substitute.
- [ ] Complete formatter/Clippy and advisory checks, or record an explicit disposition for unavailable tools. No installation is implied.
- [ ] Review notices, corresponding source, secrets, backup/migration guidance and the exact supported environment before another release. Maintainer contact is not a verified private security-reporting route.
- [ ] Before a corrected release, select a new version, align manifest/lock, qualify the exact final commit and update its notes. Obtain separate authorization for pushes, new tags and publication.

Native qualification remains incomplete. No real-model quality, physical model capacity, general installation, multi-day endurance or reboot/power-loss guarantee is established. Optional P7 endurance remains separate from the existing bounded P2–P3 contract.

## Verification history

The original preparation reviewed source baseline `ad92aea18bbf3614f32448be81a1a9e40a5a2473`, found no existing release procedure/workflow, and passed 77 contained tests with eight native tests ignored. Cargo metadata then reported `0.1.0`. Historical logs remain in `.build/alpha1-release-tests.txt` and `.build/release-metadata.json`.

At preparation time, no release was visible through the connector and the repository-contained CLI configuration was unauthenticated. Later authorized read-only access to `~/.config/gh/` found no file-backed token; credential-store access was not authorized or attempted. The owner's public release URL then allowed inspection without credentials. The revised notes were already present. No agent published or edited the GitHub release during these checks.

## Local correction validation

Build/test logs are repository-local `.build/post-release-build.txt` and `.build/post-release-tests.txt`; fresh dependency metadata is `.build/metadata.json`. The contained offline build passed. The rebuilt CLI reports `rivet 0.1.0-alpha.1 build 75e751a226fe7d51eb7ceb17699888ddf5ca07ff6d91e08bf265e86aec01d269`. The locked offline all-target suite passed **77 tests, zero failed, eight ignored**. The version-aware dependency audit passed for all **227** locked archives. Cargo metadata agrees with the manifest/lock; a parsed lockfile comparison confirms no dependency changes. Documentation checks passed for 26 local/GitHub file-link targets across four Markdown files, titles/newlines and diff whitespace. These results apply to the local correction, not retroactively to the published tag. No native or live-model checks were run; no rendered Markdown preview is claimed.

## Next release procedure

1. Choose a new identifier, normally `0.1.0-alpha.2` for this correction, and authorize its bounded preparation.
2. Align Cargo.toml and the root Cargo.lock entry, then inspect the lock diff for unrelated dependency changes.
3. Build through the contained offline route; check CLI version and digest. Run the offline/native qualification and separately approved live checks, documenting any limitations.
4. Bind evidence and notes to the exact candidate SHA. Review secrets, licenses, source correspondence and supported environment.
5. With explicit permission for each external effect, push the source, create/push the new tag, save a draft pre-release and verify its target, flags, body and assets. Publication is a separate final action.

Reference: [GitHub release API](https://docs.github.com/en/rest/releases/releases). A draft/pre-release flag is not a substitute for version alignment or qualification. The [GitHub CLI release command](https://cli.github.com/manual/gh_release_create) can create missing tags automatically, so never use it without checking the authorized tag effects.
