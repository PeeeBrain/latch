# Contributing to Latch

Thanks for contributing to Latch.

## Development setup

Install Rust through rustup and the platform prerequisites in [README.md](README.md). The root workspace contains `latch-core` and the GPUI `latch-desktop` application. There is no web frontend or Tauri host.

## Code style

Run these checks from the repository root:

```bash
cargo fmt --all
cargo check -p latch-desktop --all-targets --locked
cargo test -p latch-core --locked
cargo test -p latch-core --features legacy-oauth --locked
cargo test -p latch-desktop --locked
cargo clippy -p latch-core -p latch-desktop --all-targets --locked -- -D warnings
cargo clippy -p latch-core --all-targets --all-features --locked -- -D warnings
python -m unittest discover -s scripts -p 'test_*.py' -v
```

Release-tooling tests require Python 3.11+ and `minisign` on PATH. Keep the root `Cargo.lock` committed. The `legacy-oauth` feature covers historical vault compatibility; it must stay out of the desktop dependency graph.

Agents must follow the [no-local-build policy](AGENTS.md). Builds and package checks run on GitHub Actions, including signed packaging rehearsals. Do not run build jobs through `act` locally.

## Pull request process

1. Document user-facing changes in `CHANGELOG.md`.
2. Update the README.md with details of changes if applicable.
3. The PR will be merged once you have the sign-off of at least one maintainer.

## CI and release policy

- CI checks Windows x64, macOS ARM64/x64, and Linux x64 for changes to the crates, workspace, workflows, and release scripts. Documentation-only changes skip these jobs. If CI becomes required in branch protection, replace path filters with change detection so skipped workflows cannot leave checks pending.
- A newer commit to a pull request cancels the older CI and dependency review runs for that pull request.
- Dependency review runs on every pull request and fails when a change introduces a known vulnerable dependency, except for documented existing advisories without a compatible fix.
- The Rust dependency audit runs when Cargo manifests or the lockfile change, every Monday at 03:17 UTC, and on manual dispatch.
- Releases start only when a maintainer pushes a matching `vMAJOR.MINOR.PATCH` tag on a commit reachable from `main`. Updating Cargo versions or merging a PR never creates a release by itself. Manual dispatch retries an existing tag. Follow [the release procedure](docs/releases.md).

Commit the root `Cargo.lock` and use `--locked` in CI and release jobs.

## Commit messages

Please provide clear, concise commit messages that describe what the change does.
