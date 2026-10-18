# Latch Password Manager

A local password manager with a compact, keyboard-first GPUI interface for Windows, macOS, and Linux.

The native Rust rewrite is in progress. Vault flows are implemented, but signing, installed-app upgrades, accessibility, and platform acceptance still need validation. See the [implementation plan](docs/plans/gpui-rebuild.md) for progress and release gates.

## Run in development

The native app runs from the repository root. It does not need Bun, React, a web server, or a WebView.

Install Rust through rustup. The repository's `rust-toolchain.toml` selects Rust 1.98.1 with rustfmt and Clippy. Windows needs the MSVC C++ build tools and Windows SDK; macOS needs the Xcode command line tools. Linux needs the GPUI and GTK development packages listed in the native job in [ci.yml](.github/workflows/ci.yml).

```sh
cargo run -p latch-desktop --locked
```

This compiles and launches the current source in debug mode. Restart the command after changing Rust code. Launching an existing executable in `target/debug/` does not recompile it.

By default, development uses your normal vault. Close any other Latch host using that vault first; both the native app and the temporary migration host take an exclusive lock.

For UI work with a separate vault, set `LATCH_DEV_VAULT` to a throwaway file path. Create its parent directory before launching. In PowerShell:

```powershell
$env:LATCH_DEV_VAULT = Join-Path $env:TEMP 'latch-dev/vault.enc'
New-Item -ItemType Directory -Force (Split-Path $env:LATCH_DEV_VAULT) | Out-Null
cargo run -p latch-desktop --locked
```

On macOS/Linux:

```sh
mkdir -p /tmp/latch-dev
LATCH_DEV_VAULT=/tmp/latch-dev/vault.enc cargo run -p latch-desktop --locked
```

The override works only in debug builds. A missing vault starts setup. Appearance settings also live beside the selected vault. In PowerShell, remove the override with `Remove-Item Env:LATCH_DEV_VAULT` before returning to your normal vault.

These launch commands are for developers to run. Agents must follow the [no-local-build policy](AGENTS.md), use check/test commands, and use CI artifacts for runtime validation.

## How the app works

Latch opens one command window. Create or unlock a vault, then type to filter credentials and commands. Search, credential editing, password generation, health checks, and settings all stay inside this window. The footer names the current Enter action; the Actions panel provides commands for the selected credential.

- Press Up/Down to navigate and Enter to copy the selected credential's password or run a selected command.
- Open Actions with Ctrl+K, or Cmd+K on macOS, to view details, edit, or copy other fields.
- Use Ctrl+N to add a credential, Ctrl+E to edit, Ctrl+L to lock, and Ctrl+, to open settings. Use Cmd for these shortcuts on macOS.
- Press Escape to go back and Tab/Shift+Tab to move between controls.
- Ctrl+Space shows or hides Latch on all platforms when global shortcut registration succeeds.

Native access supports a master password and device-key access through Windows Hello or macOS Keychain on supported devices. The app also provides timed password reveal, TOTP codes, configurable password generation, SimpleLogin/DuckDuckGo aliases, local password health checks, optional HIBP breach checks, and password/device-key rotation.

The vault locks after 30 minutes of inactivity. Copied secrets clear after 30 seconds if Latch still owns that clipboard content. On Windows/macOS, closing the window locks the vault and hides it to the tray when tray initialization succeeds. On Linux, closing quits the app.

## Appearance

Open Settings, then Appearance to import a T3 Code, VS Code, or Zed theme, or editor `settings.json` color overrides. Latch keeps supported colors, previews them before applying, and converts imported themes into editor settings. Light/dark variants follow the OS appearance.

You can open, edit, reload, or reset Latch's appearance `settings.json` from the app. It lives beside the vault. There is no preset theme gallery or separate public Latch theme format. Ctrl+Alt+R or Cmd+Alt+R resets appearance if custom colors make the interface hard to read.

## Architecture and storage

`crates/latch-desktop` owns the GPUI window, Rust text input, clipboard, tray, global shortcut, platform device-key integration, and signed updater. It uses GPUI Kit's component layer for the title bar and shortcut formatting, with embedded Lucide icons.

`crates/latch-core` owns authentication, encryption, vault storage, the unlocked workspace, search, password generation, aliases, and health checks. The root Cargo workspace owns the toolchain, lockfile, and target directory.

The encrypted vault file is `vault.enc` in the OS configuration directory:

- Windows: `%APPDATA%\Latch\vault.enc`
- macOS: `~/Library/Application Support/Latch/vault.enc`
- Linux: `$XDG_CONFIG_HOME/latch/vault.enc`, defaulting to `~/.config/latch/vault.enc`

The file format is shared across platforms. Password vaults use PBKDF2-HMAC-SHA256 with 100,000 iterations and AES-256-GCM encryption. Native also reads historical Argon2id password vaults. Device-key vaults use platform key storage. The unlocked session key stays in memory and clears on lock. There is no master-password recovery.

## Existing Google vaults

Google OAuth is retired in the native app. It detects an OAuth vault and leaves it unchanged. Before using native Latch, unlock the vault in the temporary Tauri migration host and choose Settings > Switch to master password.

The migration host remains in `frontend/` and `frontend/src-tauri/`. It requires Bun 1.4.2, the repository Rust toolchain, and Tauri platform prerequisites. To launch it:

```sh
cd frontend
bun install
bun run tauri dev
```

Close it before opening the same vault in the native app. Only the migration host enables `latch-core/legacy-oauth`; the native dependency graph excludes Tauri and Google/JWT processing.

## CI previews and packages

Pull-request CI uploads `gpui-preview-<OS>-<architecture>` artifacts with a release-mode executable. Download and extract the artifact for your platform, then launch `latch-desktop.exe` on Windows or `latch-desktop` on macOS/Linux. These previews use your normal vault location and are not signed installers.

On macOS/Linux, restore executable permission with `chmod +x latch-desktop`. Windows debug executables depend on shader files on their build machine; CI release previews embed those shaders and are portable.

The Native GPUI packages workflow uploads `native-<OS>-<architecture>` artifacts containing Windows NSIS, macOS app/DMG, and Linux deb/AppImage packages. The deb targets Ubuntu 24.04; AppImage needs the host's Wayland and GPU drivers. Signing and installed-app upgrade acceptance remain release gates. The native signed updater uses a separate feed from the migration app.

## Contributing and checks

See [AGENTS.md](AGENTS.md) for repository policies and [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidance. From the repository root, run:

```sh
cargo fmt --all -- --check
cargo check -p latch-desktop --all-targets --locked
cargo test -p latch-core --locked
cargo test -p latch-core --features legacy-oauth --locked
cargo test -p latch-desktop --locked
cargo clippy -p latch-core -p latch-desktop --all-targets --locked -- -D warnings
```

Do not use `--workspace --all-features` to verify native OAuth removal; that enables the migration host's legacy feature. Migration-host and frontend changes also need the checks listed in AGENTS.md. Builds and packaging run in GitHub Actions. Native launch, GPU, accessibility, and installer validation need runtime checks on each supported platform.

## Troubleshooting

- An old interface usually means an old executable is running. Close it and restart with `cargo run -p latch-desktop --locked`.
- If another process holds the vault lock, close the other Latch host or choose a separate development vault.
- If Ctrl+Space is unavailable, another application or the desktop environment may own it. Open Latch from the application launcher.
- A Google vault requires migration through the temporary host. A damaged or unsupported vault is preserved rather than replaced with an empty one.

## License

MIT. See [LICENSE](LICENSE).
