# Build from source

Run commands from the repository root. Current qualification uses Linux x86_64, Rust/Cargo 1.99.1, an existing C compiler/linker and bubblewrap. Runtime workers additionally require a qualified user-systemd/cgroup v2 and strict Landlock profile. Missing enforcement denies workers; a successful build is not worker readiness.

The existing developer runner exposes only the checkout and required read-only system build directories, hides host configuration/credentials and keeps caches, temporary files and output under `.build/`. Its paths are tailored to the qualified environment; other Linux distributions are unverified. Inspect the runner before granting its use. It is not a product installer.

```bash
scripts/cargo-contained probe
scripts/cargo-contained offline build --locked --offline
scripts/cargo-contained offline check --locked --offline --all-targets
scripts/cargo-contained offline test --locked --offline --all-targets
```

Offline compilation requires the locked source dependencies already cached locally. If they are absent, stop. `scripts/cargo-contained fetch --locked` acquires dependency sources over the network and needs explicit authorization under repository policy. It does not authorize installing a compiler, model or inference runtime. No fetch was run for the post-P3 audit.

The executable is `.build/target/debug/rivet`. Its current root validation and worker bootstrap bind it to the build/source checkout, and workers use that debug path. Copying it elsewhere or using it for arbitrary external projects is not qualified. There is no verified package-manager, release-download, portable-install or uninstall procedure yet.

Eight tests intentionally require native systemd access and are ignored in containment. `python3 scripts/test-native.py` is for a separately authorized native qualification run: it creates temporary units and uses host cgroups. Do not mistake ignored tests for passed native acceptance.

Rustfmt and Clippy were unavailable in prior qualification; see the latest [audit](../planning/research/post-p3-verification.md) for current results. No component installation is implied.
