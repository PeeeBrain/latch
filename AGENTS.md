# AGENTS.md

This file provides guidelines for AI agents working on Latch Password Manager codebase.

**Available jobs:**
- `frontend`: TypeScript typecheck, ESLint, and Vite build
- `backend`: Rust fmt check, compile check, tests, and clippy
- `cargo-audit`: Security vulnerability scan (runs automatically in full workflow)

**Best practice**: Run `act` before pushing to catch CI failures early and avoid round-trip debugging.

### Pre-PR CI Preparation Checklist

**MANDATORY: Run these commands before creating any PR to ensure CI passes:**

#### Backend (Rust)
```bash
cd frontend/src-tauri

# 1. Format code (CI will fail if not formatted)
cargo fmt --all

# 2. Check compilation
cargo check

# 3. Run clippy with strict warnings (CI uses: -D warnings)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Run all tests
cargo test
```

#### Frontend (TypeScript)
```bash
cd frontend

# 1. Type check
bun run typecheck

# 2. Lint
bun run lint

# 3. Run tests
bun run test
```

**Common CI Failures to Avoid:**
- ❌ `cargo fmt` not run → Formatting errors
- ❌ Dead code (unused functions/variables) → Clippy `-D warnings` fails
- ❌ Missing `mut` or extra `mut` → Compiler errors
- ❌ Import errors → TypeScript build fails
- ❌ Test failures → `cargo test` fails

**Rule of Thumb**: If any command above fails locally, the CI will fail. Fix it before pushing.

## Code Style Guidelines

### Architecture Principles

1. **Strict Layer Separation**: Frontend=UI, Rust backend=crypto/storage/auth
2. **Tauri State Pattern**: Vault instance stored in `VaultState(Mutex<Vault>)`, shared via Tauri State
3. **Auth Method Flexibility**: Support multiple auth methods (OAuth, Biometric) with method switching
4. **Master Password Security**: For biometric vaults, key stored in OS keychain (Windows Credential Manager/macOS Keychain); for OAuth, derived from Google user_id
5. **Session-Only Keys**: Decrypted vault key lives in memory only, cleared on lock or 30min timeout
6. **Command Palette UI**: Raycast-style single-window interface with mode-based navigation


## Important Notes

1. **Never commit secrets** - Check .gitignore
2. **No sidecars** - Use Tauri State, not `app.shell().sidecar()`
3. **Status field** - JSON responses must include `status: "success"`
4. **Session management** - Vault auto-locks after 30 minutes
5. **No password recovery** - Forgotten master password = lost data (by design)
6. **Cross-platform** - Vault file format identical across Windows/macOS/Linux
7. **No-build policy** - NEVER RUN `cargo build` locally or `bun run build` ever.

### Command Palette Pattern
The app uses a Raycast-style single-window command palette interface with mode-based navigation
