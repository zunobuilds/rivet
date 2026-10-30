# Versioning policy

Rivet follows **[Semantic Versioning 2.0.0](https://semver.org/)** as its official versioning standard. Versions use `MAJOR.MINOR.PATCH`, with optional pre-release identifiers and build metadata. A version number does not certify production readiness.

## Compatibility surface

Rivet's public interface for version decisions is its documented CLI commands/flags, exit codes, structured output, configuration and accepted plan/policy formats. See [configuration](configuration.md), [operations](operations.md) and [modes](modes.md). State-schema compatibility and required migration/recovery steps must be documented per release. Internal Rust modules and worker protocols are not a separately supported library API.

## Development and stable versions

During `0.x` development, interfaces can change. Rivet's convention is to use a new minor version for incompatible changes or a new development milestone, and a patch version for compatible fixes within that milestone. Release notes must identify breaking changes and migration limits; a shared `0.x` prefix is not a compatibility guarantee.

At `1.0.0`, the public interface becomes stable: incompatible changes require a major increment, compatible additions/deprecations a minor increment, and compatible fixes a patch increment. Reset lower components when increasing a higher component. Published versions and their tags must never be rewritten; corrections need a new version.

## Pre-release progression

| Stage | Rivet's meaning |
| --- | --- |
| `alpha.N` | Early experimental milestone; interfaces and scope may change substantially. |
| `beta.N` | Intended milestone scope is implemented; focus shifts to compatibility and qualification. |
| `rc.N` | Release candidate: planned qualification is complete, with only release-blocking fixes expected. |
| No suffix | Normal release of the milestone; `0.x` still carries development-stage instability. |

These stage meanings are Rivet policy, not maturity guarantees supplied by SemVer. Begin each stage counter at 1. Progress only after its evidence gates pass; changing the suffix alone does not advance readiness.

Example ordering: `0.1.0-alpha.1 < 0.1.0-alpha.2 < 0.1.0-beta.1 < 0.1.0-rc.1 < 0.1.0`. Numeric identifiers have no leading zeros. Optional `+build` metadata does not affect precedence; Rivet's existing build digest remains a diagnostic identity, not a replacement for the package version.

## Release identity

Use package version `0.1.0-alpha.1`, Git tag `v0.1.0-alpha.1` and release title `Rivet v0.1.0-alpha.1`. The `v` belongs to the tag/title, not Cargo's version. Cargo.toml, the root Cargo.lock entry, rebuilt CLI output and release notes must agree before publication. Reserve one immutable source commit per released version.

GitHub draft status controls publication; its pre-release flag labels the release. Both are separate from the SemVer suffix. Tag creation, pushes and publication each require explicit authorization. See [release readiness](release-readiness.md) for the published alpha’s version discrepancy and correction plan.
