# Latch

Latch is a local, keyboard-first password manager for Windows, macOS, and Linux. One Raycast-style command window handles search, copy, edit, generate, health checks, and settings. The vault is a single encrypted file on disk. There is no cloud, no account, and no sidecar.

### 1. Local and private by default

The vault never leaves the machine unless the user moves it. Decrypted keys live in memory only and are cleared on lock or after 30 minutes. Network calls are rare, opt-in where possible, and never carry vault data (favicons, HIBP, alias providers). There is no password recovery, by design. Losing the master password means losing the data.

### 2. Fast and native

Latch is a native GPUI app with no WebView and no runtime. It should open, filter, and copy faster than the user can think about it. Watch for redundant re-renders, blocking I/O on the UI thread, and continuously repainting animations. Every change should consider its performance cost.

### 3. Compatible forever

The vault format is identical on every OS, and old vaults must keep opening. Preserve vault paths, app identity, and OS device-key identifiers. Breaking a user's vault is the worst bug Latch can ship.

## A note from Partha

I like ambitious ideas, simple systems, and software that feels obvious. Do not preserve complexity just because it already exists. Do not introduce machinery because it looks architecturally impressive. Understand the real constraint, then fight for the smallest model that makes the correct behavior unsurprising.

Channel both "measure twice, cut once" and "yagni". Fight scope creep. Try to honor the dev's intent in both a minimal and realistic fashion.

The one place to be paranoid rather than minimal is the vault. Crypto, key handling, file writes, and migrations get extra care and a second look. A simple system that loses someone's passwords is not simple, it's broken.

The rest of this document is meant to help you navigate the codebase and make changes effectively. Think of these instructions less as "hard rules", more as "good defaults". The developer's preferences should be able to override anything here.

## A small glossary

We need to be on the same page with terminology. When communicating, use this language:

- **you** means the agent reading this file and changing Latch.
- **we, us, and maintainers** mean Partha and anyone maintaining Latch. These are who you are talking to now.
- **user** means the person running the Latch desktop app.
- **Vault** means the encrypted file on disk (`vault.enc`): auth metadata plus ciphertext. Not "store", "database", or "file".
- **Workspace** means the in-memory decrypted state (Credentials plus the session key). It exists only during a Session. Never call it "vault".
- **Credential** means one stored login. Not "entry", "record", or "item".
- **AuthMethod** means how a Vault is provisioned and accessed: `Password` (master password + PBKDF2) or `Biometric`, which users see as "device key" (Windows Hello / macOS Keychain).
- **Provision / Access / Rotate**: create a new Vault / unlock an existing one into a Workspace / re-encrypt with a new key or AuthMethod without touching Credentials.
- **Session** means the 30-minute window during which a Workspace exists. It auto-locks on expiry, on lock, or on window close (Windows/macOS).
- **Lockout** means exponential backoff after failed Access: 5s base, doubling, 5 min max.
- **VaultCoordinator** means the core type (`latch-core/src/vault/coordinator.rs`) that owns Vault access and the Session lifecycle. The UI goes through it, never around it.
- **VaultHealth** means weak, reused, and breached checks across all Credentials. Breach checks (HIBP) are optional and network-bound.
- **ClipboardGuard** means copy-then-clear after 30 seconds, but only if Latch still owns the clipboard.
- **Command Palette** means the single Raycast-style window. Its screens are **Pages** (`Page` enum in `latch-desktop/src/app.rs`: Browse, Actions, Edit, Settings, Generator, Health, …).
- **Appearance** means user colors imported from VS Code/Zed/T3 Code themes into an editor-style `settings.json` beside the Vault. It is not a Latch theme format.

## Running locally

- Everything runs from the repo root. The root Cargo workspace owns `Cargo.lock`, the toolchain (`rust-toolchain.toml`), and `target/`. Always pass `--locked`.
- **Do not build or launch the app.** No `cargo build`, `cargo run`, `bun run build`, or packaging, and do not run build jobs through `act`. Use `cargo check` and `cargo test` locally; CI artifacts own native launch and package verification.
- When the developer runs the app for UI work, they point it at a throwaway vault with `LATCH_DEV_VAULT` (debug builds only). Never touch their real vault.
- Latch takes an exclusive lock on the Vault. If a check or test fails on a lock, another Latch host is probably running. Say so; don't kill it.
- Stop only processes you started, by the PID you tracked. Never kill Latch or cargo by name.

## Verifying

- Smallest proof that the change works. While iterating, check and test only the crate you touched: `cargo check -p <crate> --all-targets --locked`, `cargo test -p <crate> --locked [test_name]`.
- Don't add tests by default. A behavior change in crypto, vault I/O, auth, Session, or migrations gets one focused test for that behavior; anything else, ask first. Never delete or weaken an existing test to make it pass.
- UI changes have no local runtime check. Say what the developer should look at in a CI artifact or their own `cargo run`.
- Before a PR, run the full checklist from the repo root. CI runs the same commands with `-D warnings`, so anything that fails here fails there:

  ```bash
  cargo fmt --all
  cargo check -p latch-desktop --all-targets --locked
  cargo clippy -p latch-core -p latch-desktop --all-targets --locked -- -D warnings
  cargo test -p latch-core --locked
  cargo test -p latch-desktop --locked
  ```

- CI (`ci.yml`) runs path-filtered jobs: `native` (fmt/check/clippy/tests on Windows, macOS, Linux), `release-tooling` (release script tests, when `scripts/` changes), and `workflows` (actionlint, when workflows change), gated by `CI status`. `security.yml` runs `cargo audit`. You don't need to run the release or workflow checks locally.

## Pull requests

- Never make a PR unless the developer explicitly asks you to.
- Conventional commit titles, plain language: `fix(desktop): search no longer drops the selection after a copy`. Scopes are `core`, `desktop`, `ci`, `release`, `docs`.
- Body: the problem in a sentence or two, then how you fixed it. Call out anything touching the Vault format, crypto, AuthMethod, device-key identifiers, Session/lock timing, or network calls. End with the model and harness that did the work.
- **Rebase onto latest main before opening.** Stale branches conflict and burn a review round.
- UI changes need before/after screenshots. Motion or timing needs a short video.
- One concern per PR. If the description says "also", split it.
- Add user-visible changes under `## [Unreleased]` in `CHANGELOG.md`. Update `README.md` only if documented behavior changed.
- Do not bump Cargo versions in ordinary PRs. Releases start only from a maintainer-pushed `vX.Y.Z` tag on `main` (see `docs/releases.md`).
- When babysitting: poll checks and comments newer than the last push, verify each bot finding against the source, fix real ones, dismiss false positives with a written reason. Stay quiet when nothing is new. Stop when checks are green on the latest commit.

## How it works

`latch-desktop` starts one GPUI window, the **Command Palette**, plus a tray icon and a global Ctrl/Cmd+Space hotkey (`integration.rs`). The `Latch` app state in `app.rs` switches between **Pages**; `app/views.rs` renders them.

Every Vault operation goes through `latch-core`'s **VaultCoordinator**. On **Access** it checks **Lockout**, reads `vault.enc` via `VaultStorage`, derives the key with the Vault's **AuthMethod**, decrypts into the **Workspace**, and schedules **Session** expiry. All later reads and writes go through `with_vault`, so Session timing stays in one place. On lock or expiry the Workspace and key are dropped (`zeroize`).

**Device key** (`biometric.rs`) stores the Vault key in Windows Hello / macOS Keychain under fixed identifiers. Changing those identifiers locks users out.

The network is touched only at the edges: favicons (`favicon.rs`, direct HTTPS, no private-network targets), optional HIBP breach checks (`vault_health/breach_checker.rs`), email alias providers (`vault/alias.rs`), and the signed updater (`update.rs`, pinned public key, separate from the retired Tauri feed). None of these send Vault data.

Deeper decisions live in `CONTEXT.md` and `docs/adr/` (AES-256-GCM, KDF per AuthMethod).

## Where code lives

- `crates/latch-core/` holds everything that touches secrets. No UI, no GPUI.
  - `auth/` covers AuthMethod, key derivation, password KDF, and Lockout.
  - `crypto/` is AES-256-GCM (`aead.rs`).
  - `vault/` is the VaultCoordinator, storage, Provision/Access/Rotate, Workspace, Credential CRUD (`entries.rs`), search, TOTP, and email aliases.
  - `vault_health/` holds the weak/reused audit and the HIBP breach checker.
  - `password_generator.rs`
- `crates/latch-desktop/` is the GPUI host. Almost all UI work happens here.
  - `app.rs` holds `Latch` app state, the `Page` enum, and actions. `app/views.rs` renders Pages.
  - `ui.rs` holds shell design tokens; `input.rs` the text input (masked values never enter shaped text); `theme.rs` Appearance import and `settings.json`.
  - `integration.rs` covers tray and global hotkey; `biometric.rs` device key; `clipboard.rs` ClipboardGuard; `favicon.rs` icon fetching; `update.rs` the signed updater.
  - `updater.pub` is the updater trust key. Never replace it. `icons/` and `build.rs` handle packaging assets.
- `CONTEXT.md` is the domain language. Update it when terms or boundaries change.
- `docs/adr/` holds architecture decisions; `docs/releases.md` the release procedure.
- `scripts/release.py` is release tooling used by `release.yml`.
- `.github/workflows/` covers CI, packaging, release, and security. `.github/ci-filters.yml` sets which paths trigger which jobs.

## Taste

- Complexity belongs in `latch-core`: crypto, storage, auth, Session. `latch-desktop` stays a thin host that renders Pages and calls the VaultCoordinator. It never derives keys, touches `vault.enc`, or tracks Session time itself.
- Keyboard first. Every flow works without a mouse, Enter does the obvious thing, Escape goes back, and the footer tells the truth about what Enter will do. Preserve the compact Raycast feel when redesigning.
- When designing or redesigning a UI surface, load the `apple-design` and `mobile-native` skills first so layout, motion, and interaction choices are deliberate by default.
- Secrets stay out of `String`s that outlive their use, logs, error messages, and shaped text. Use `Zeroizing` for keys and passwords.
- Errors shown to users are short and plain, and never reveal which half of a credential was wrong or whether the vault is damaged versus the key wrong (see `coordinator.rs`).
- No new dependencies for what a few lines can do. Every crate in a password manager is attack surface.
- Comments describe how a thing is used, and move when the code moves. Short notes on functions and seams, not play-by-play.
- Users live in other apps all day. They notice a hotkey that misses, a clipboard that doesn't clear, a lock that fires mid-edit, or a window that repaints when idle.
- Never commit secrets, test vaults, or real credentials.
- If a rule here fights the task in front of you, say so loudly and get a human sign-off before breaking it.
