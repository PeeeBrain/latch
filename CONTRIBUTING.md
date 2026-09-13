# Contributing to Latch

Thanks for contributing to Latch.

## Development setup

1. Make sure you have Node.js, bun, and Rust installed.
2. Clone the repository.
3. Install frontend dependencies:
   ```bash
   cd frontend
   bun install
   ```
4. Start the development server:
   ```bash
   bun dev
   ```

## Code style

Run these frontend checks from `frontend/`:

```bash
bun run typecheck
bun run lint
bun run test
```

Run these backend checks from `frontend/src-tauri/`:

```bash
cargo fmt --all
cargo check --all-targets --locked
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
```

## Pull request process

1. Document user-facing changes in `CHANGELOG.md`.
2. Update the README.md with details of changes if applicable.
3. The PR will be merged once you have the sign-off of at least one maintainer.

## CI and release policy

- CI runs for changes under `frontend/` and for changes to the CI workflow. Documentation-only pull requests do not start frontend or backend jobs. If CI becomes a required check in branch protection, replace the workflow path filters with a lightweight change-detection job so skipped workflows cannot leave pull requests pending.
- A newer commit to a pull request cancels the older CI and dependency review runs for that pull request.
- Dependency review runs on every pull request and fails when a change introduces a known vulnerable dependency, except for documented existing advisories without a compatible fix.
- Full Rust and Bun dependency audits run when either lockfile changes, every Monday at 03:17 UTC, and when a maintainer starts the workflow manually.
- Releases run only from a version tag such as `v0.2.5`. The tag must match the version in `frontend/src-tauri/tauri.conf.json`. Manual dispatch exists to retry an existing tag.
- Keep the 4-vCPU Blacksmith runners until job timing data shows that another size will reduce cost or elapsed time.

Commit `frontend/bun.lock` and `frontend/src-tauri/Cargo.lock`. Install dependencies with the lockfiles frozen in CI and release jobs.

## Commit messages

Please provide clear, concise commit messages that describe what the change does.
