# Latch Password Manager

A local password manager with a compact, keyboard-first GPUI interface for Windows, macOS, and Linux.

The native Rust rewrite replaces the previous Tauri app. See [downloads and upgrades](#downloads-and-upgrades) for installation and migration, and the [implementation plan](docs/plans/gpui-rebuild.md) for platform acceptance work.

## Run in development

The native app runs from the repository root. It does not need Bun, React, a web server, or a WebView.

Install Rust through rustup. The repository's `rust-toolchain.toml` selects Rust 1.98.1 with rustfmt and Clippy. Windows needs the MSVC C++ build tools and Windows SDK; macOS needs the Xcode command line tools. Linux needs the GPUI and GTK development packages listed in the native job in [ci.yml](.github/workflows/ci.yml).

```sh
cargo run -p latch-desktop --locked
```

This compiles and launches the current source in debug mode. Restart the command after changing Rust code. Launching an existing executable in `target/debug/` does not recompile it.

By default, development uses your normal vault. Close any other Latch host using that vault first; Latch takes an exclusive lock to prevent concurrent writes.

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

Credential rows and details load favicons directly from the saved website's HTTPS `/favicon.ico`, including redirects to public HTTPS image hosts. Latch strips the saved URL's path and query, sends no vault credentials or cookies, and uses no third-party favicon service. Icons and failed lookups stay in memory until lock. Missing URLs, unavailable icons, and private-network sites keep their initial tile. Requests bypass system proxies so destination checks apply to the actual connection. Sites that require a proxy or only declare icons in HTML keep their initial tile too.

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

## Downloads and upgrades

Download installers from the [latest release](https://github.com/PeeeBrain/latch/releases/latest). Asset names include the version, operating system, and architecture:

- Windows x64: run the `windows-x86_64-setup.exe` installer.
- macOS Apple Silicon: open the `macos-aarch64.dmg` and drag Latch to Applications.
- macOS Intel: use the `macos-x86_64.dmg`.
- Linux x64: use the `linux-x86_64.AppImage`, run `chmod +x` on it, then launch it. Ubuntu 24.04 users can install the `linux-x86_64.deb` with `sudo apt install ./<downloaded-file>.deb`.

The deb requires Ubuntu 24.04 or compatible runtime libraries. AppImage also requires compatible host Wayland and GPU drivers. SHA-256 hashes are in `SHA256SUMS`. The `.sig` files and `.app.tar.gz` downloads support the updater; macOS users should install from the DMG.

Release packages have Minisign signatures for the in-app updater. They are currently not Windows Authenticode signed or Apple Developer ID signed/notarized. Windows may show an unknown-publisher prompt, and macOS may block the app until you approve it through System Settings > Privacy & Security. Only approve a download you trust. OS signing and device/accessibility acceptance remain platform validation work.

### Existing installations

The first GPUI release requires manual installation for users of the previous Tauri app. The new packages are not offered through its `latest.json` updater feed. Quit the old app fully, including its tray process, and back up `vault.enc` before installing. On Windows, uninstall the previous Latch application before running the native NSIS installer, especially if you used the MSI. On macOS, replace the previous Latch.app in Applications. Keep your vault file and OS keychain entries.

Google OAuth is retired. Before removing the previous app, unlock any Google vault there and choose Settings > Switch to master password. The native app detects an unmigrated Google vault and leaves it unchanged. The [previous v0.2.6 release](https://github.com/PeeeBrain/latch/releases/tag/v0.2.6) remains available for migration; the old host's source is available at its tag. Close it before opening the same vault in the native app. Retain access through the previous installation until you have verified the new app can unlock your vault.

### Native updates

Installed native Windows/macOS apps and Linux AppImage launches use the signed `native-latest.json` feed through Settings > Check for updates. Installation locks the vault, verifies the download, installs, and exits Latch. Reopen the app afterward. Linux deb installations require downloading and installing the newer deb; in-app installation is disabled for them.

## CI previews and packages

Pull-request CI uploads `gpui-preview-<OS>-<architecture>` artifacts with a release-mode executable. Download and extract the artifact for your platform, then launch `latch-desktop.exe` on Windows or `latch-desktop` on macOS/Linux. These previews use your normal vault location and are not signed installers.

On macOS/Linux, restore executable permission with `chmod +x latch-desktop`. Windows debug executables depend on shader files on their build machine; CI release previews embed those shaders and are portable.

The Native GPUI packages workflow uploads Windows NSIS, macOS app/DMG, and Linux deb/AppImage artifacts. Maintainers can dispatch it on a branch with `sign` enabled to rehearse release packaging without publishing. See [the release procedure](docs/releases.md).

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

The `legacy-oauth` feature exists only for historical vault compatibility tests. The desktop dependency must never enable it. Builds and packaging run in GitHub Actions. Native launch, GPU, accessibility, and installer validation need runtime checks on each supported platform.

## Troubleshooting

- An old interface usually means an old executable is running. Close it and restart with `cargo run -p latch-desktop --locked`.
- If another process holds the vault lock, close the other Latch host or choose a separate development vault.
- If Ctrl+Space is unavailable, another application or the desktop environment may own it. Open Latch from the application launcher.
- A Google vault requires migration through the previous app before upgrading. A damaged or unsupported vault is preserved rather than replaced with an empty one.

## License

MIT. See [LICENSE](LICENSE).
