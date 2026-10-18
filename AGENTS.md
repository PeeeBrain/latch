# AGENTS.md

This file provides guidelines for AI agents working on Latch Password Manager codebase.

**Available jobs:**
- `native`: GPUI compile checks, tests, and strict Clippy on Windows, macOS, and Linux
- `release-tooling`: tag, asset, signature, and updater-manifest contract tests
- `dependency-audit`: Rust security vulnerability scan

**Best practice**: Validate workflows with actionlint. Use CI artifacts for native build and package verification; do not execute build jobs through `act` locally.

### Pre-PR CI Preparation Checklist

**MANDATORY: Run these commands before creating any PR to ensure CI passes:**

Run from the repository root:
```bash
# 1. Format code (CI will fail if not formatted)
cargo fmt --all

# 2. Check compilation
cargo check -p latch-desktop --all-targets --locked

# 3. Run clippy with strict warnings (CI uses: -D warnings)
cargo clippy -p latch-core -p latch-desktop --all-targets --locked -- -D warnings
cargo clippy -p latch-core --all-targets --all-features --locked -- -D warnings

# 4. Run all tests
cargo test -p latch-core --locked
cargo test -p latch-core --features legacy-oauth --locked
cargo test -p latch-desktop --locked
```

#### Release tooling
```bash
python -m unittest discover -s scripts -p 'test_*.py' -v
```

Release-tooling tests require Python 3.11+ and `minisign` on PATH.

**Common CI Failures to Avoid:**
- ❌ `cargo fmt` not run → Formatting errors
- ❌ Dead code (unused functions/variables) → Clippy `-D warnings` fails
- ❌ Missing `mut` or extra `mut` → Compiler errors
- ❌ Test failures → `cargo test` fails

**Rule of Thumb**: If any command above fails locally, the CI will fail. Fix it before pushing.

## Code Style Guidelines

### GPUI transition

- Shared Rust code lives in `crates/latch-core`; the GPUI host lives in `crates/latch-desktop`.
- The root Cargo workspace owns `Cargo.lock`, the toolchain, and `target/`.
- Preserve the compact keyboard-first Raycast-style interaction model while redesigning flows.
- Google OAuth is retired. `latch-core/legacy-oauth` remains only for historical vault compatibility tests and must never be enabled by the desktop dependency.
- Native appearance uses existing VS Code/Zed settings keys, with no preset theme gallery or public Latch theme schema.
- Check native code with `cargo check -p latch-desktop --all-targets --locked`; run core tests with and without `--features legacy-oauth`.
- Do not use `--workspace --all-features` to verify OAuth removal: it enables the compatibility feature. Verify the native dependency graph separately.
- The no-build policy below still applies. Use CI artifacts for native launch/package checks.

### Architecture Principles

1. **Strict Layer Separation**: GPUI desktop=UI, core=crypto/storage/auth
2. **Vault coordination**: Share vault access through the core coordinator and its session lifecycle
3. **Auth methods**: Support password and device-key access with method switching; preserve unmigrated Google vaults
4. **Master Password Security**: For device-key vaults, keys live in OS key storage on supported Windows/macOS devices
5. **Session-Only Keys**: Decrypted vault key lives in memory only, cleared on lock or 30min timeout
6. **Command Palette UI**: Raycast-style single-window interface with mode-based navigation


## Important Notes

1. **Never commit secrets** - Check .gitignore
2. **No sidecars** - Use the native application and core directly
3. **Release identity** - Preserve vault paths, app identity, and OS device-key identifiers; keep native and retired updater feeds separate
4. **Session management** - Vault auto-locks after 30 minutes
5. **No password recovery** - Forgotten master password = lost data (by design)
6. **Cross-platform** - Vault file format identical across Windows/macOS/Linux
7. **No-build policy** - NEVER RUN `cargo build` locally or `bun run build` ever.

### Command Palette Pattern
The app uses a Raycast-style single-window command palette interface with mode-based navigation
