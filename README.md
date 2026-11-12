# Latch Password Manager

A local, keyboard-first password manager for Windows, macOS, and Linux. One compact window handles search, copy, edit, password generation, health checks, and settings.

Your vault is a single encrypted file on your machine. There is no cloud, no account, and no password recovery: if you lose your master password, the data is gone.

## Install

Download the package for your platform from the [latest release](https://github.com/PeeeBrain/latch/releases/latest):

| Platform | File | How to install |
| --- | --- | --- |
| Windows x64 | `Latch_<version>_windows-x86_64-setup.exe` | Run the installer. |
| macOS Apple Silicon | `Latch_<version>_macos-aarch64.dmg` | Open it and drag Latch to Applications. |
| Linux x64 (Ubuntu 24.04+, Debian-based) | `Latch_<version>_linux-x86_64.deb` | `sudo apt install ./Latch_<version>_linux-x86_64.deb` |
| Linux x64 (any distro) | `Latch_<version>_linux-x86_64.AppImage` | `chmod +x` the file, then run it. |

On Linux, finish [Linux setup](#linux-setup) before first use, so that Ctrl+Space and fingerprint unlock work.

Packages are not Windows Authenticode signed or Apple Developer ID signed and notarized. Windows may show an unknown-publisher prompt, and macOS may block Latch until you allow it in System Settings > Privacy & Security. Only approve a download you trust. Check it against `SHA256SUMS` from the same release. The `.sig` and `.app.tar.gz` files are for the in-app updater; you don't need them.

Coming from the old Tauri version of Latch? Read [Upgrading from the Tauri app](#upgrading-from-the-tauri-app) first.

## Linux setup

### Before you install

- **AppImage:** needs FUSE 2. If it doesn't start, install `libfuse2t64` (Ubuntu 24.04+) or `fuse-libs` (Fedora).
- **deb:** needs Ubuntu 24.04 or newer, or a Debian-based distro with the same library packages. On other distros, use the AppImage.
- **Device-key unlock:** needs a Secret Service keyring, which GNOME Keyring and KWallet both provide. Most desktops already run one.

### Ctrl+Space to show and hide Latch

On Linux, Latch asks your desktop for the shortcut through the XDG GlobalShortcuts portal, the only way to get one on Wayland. The portal only grants shortcuts to apps it can identify from a desktop file named `com.latch.passwordmanager.desktop`. Current packages don't install that file yet, so create it once:

```sh
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/com.latch.passwordmanager.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Latch
Exec=latch-desktop
NoDisplay=true
EOF
```

For the AppImage, replace `Exec=latch-desktop` with the full path to your `.AppImage` file, and update it if you move the file. `NoDisplay=true` keeps it out of your app launcher.

Then:

1. Start (or restart) Latch.
2. Your desktop asks whether Latch may register "Show or hide Latch" on Ctrl+Space. Allow it.
3. Open Latch's Settings. It shows the key your desktop bound. If it says to assign a key, set one for Latch in your desktop's shortcut settings (on KDE: System Settings > Keyboard > Shortcuts). Latch picks up the change without a restart.

This has been tested on KDE Plasma 6. Other desktops work if their portal supports global shortcuts. If yours doesn't, Settings says the shortcut is unavailable; open Latch from your app launcher instead. On X11 without the portal, Latch registers Ctrl+Space directly.

### Fingerprint or login-password unlock (device key)

On Linux, device-key unlock asks for your fingerprint or login password through a polkit prompt, and keeps the vault key in your login keyring. It needs Latch's polkit policy:

- **deb:** installed for you.
- **AppImage:** install it once:

  ```sh
  sudo curl -fsSL -o /usr/share/polkit-1/actions/com.latch.vault.policy https://raw.githubusercontent.com/PeeeBrain/latch/main/crates/latch-desktop/linux/com.latch.vault.policy
  ```

To use a fingerprint instead of your login password, enroll one with `fprintd-enroll`, then enable fingerprint login for your distro:

- Fedora: `sudo authselect enable-feature with-fingerprint`
- Ubuntu: `sudo pam-auth-update`

On Linux the fingerprint is a presence check that guards against someone at your unlocked machine. It is not a hardware-bound key as on Windows and macOS; see [ADR 0003](docs/adr/0003-linux-device-key.md).

### Other Linux differences

- Closing the window quits Latch, and the vault locks.
- In-app updates work only when you run the AppImage. To update a deb, download and install the newer deb.

## Using Latch

Latch opens one command window. Create or unlock a vault, then type to filter credentials and commands. The footer always shows what Enter will do.

| Action | Windows / Linux | macOS |
| --- | --- | --- |
| Show or hide Latch from any app | Ctrl+Space | Ctrl+Space |
| Move through the list | Up / Down | Up / Down |
| Copy the selected password, or run the selected command | Enter | Enter |
| Actions for the selected credential (details, edit, copy other fields) | Ctrl+K | Cmd+K |
| New credential | Ctrl+N | Cmd+N |
| Edit selected credential | Ctrl+Shift+E | Cmd+Shift+E |
| Save credential | Ctrl+S, or Enter in a text field | Cmd+S, or Enter in a text field |
| Lock | Ctrl+L | Cmd+L |
| Settings | Ctrl+, | Cmd+, |
| Reset appearance | Ctrl+Alt+R | Cmd+Alt+R |
| Go back | Escape | Escape |
| Move between controls | Tab / Shift+Tab | Tab / Shift+Tab |

You can unlock with a master password or a device key: Windows Hello, macOS Keychain, or on Linux the polkit prompt described above. Latch also has timed password reveal, TOTP codes, a configurable password generator, SimpleLogin and DuckDuckGo email aliases, local password health checks, optional breach checks, and switching between master password and device key.

**Timers.** The vault locks after 30 minutes without activity. Copied secrets clear from the clipboard after 30 seconds, unless something else has been copied since.

**Closing the window.** On Windows and macOS, closing locks the vault and hides Latch to the tray. Open it again from the tray, with Ctrl+Space, or by launching it again. On Windows, minimizing also hides it from the taskbar, and Quit is in the tray menu. On Linux, closing quits.

**Appearance.** Settings > Appearance imports colors from a T3 Code, VS Code, or Zed theme, or from editor `settings.json` color overrides, and previews them before applying. Latch stores the result in its own `settings.json` beside the vault, which you can open, edit, reload, or reset from Settings. Light and dark variants follow the OS. If custom colors make Latch hard to read, press Ctrl+Alt+R (Cmd+Alt+R on macOS) to reset.

## Privacy and network use

Latch works offline. It goes online only for these, and none of them send vault data:

- **Favicons.** Credential icons load directly from the saved site's `https://<host>/favicon.ico`, with the URL's path and query removed and no cookies or credentials sent. No third-party icon service is used, and private-network addresses are never contacted. Requests bypass system proxies so this check applies to the real connection. Icons stay in memory until you lock.
- **Breach checks (optional).** Health checks can ask Have I Been Pwned whether a password appears in known breaches. Only a short prefix of the password's SHA-1 hash leaves your machine.
- **Email aliases (optional).** SimpleLogin and DuckDuckGo are contacted only when you create an alias.
- **Updates.** Settings > Check for updates downloads the signed update feed from GitHub. Latch verifies the signature before installing.

## Where your vault lives

Latch stores everything in `vault.enc`:

- Windows: `%APPDATA%\Latch\vault.enc`
- macOS: `~/Library/Application Support/Latch/vault.enc`
- Linux: `$XDG_CONFIG_HOME/latch/vault.enc`, which defaults to `~/.config/latch/vault.enc`

The file format is the same on every OS, so you can move a password vault between machines. Password vaults use PBKDF2-HMAC-SHA256 (100,000 iterations) and AES-256-GCM, and older Argon2id vaults still open. Device-key vaults keep their key in the OS key store, so they open only on the machine and account that created them. The unlocked key stays in memory and is cleared on lock.

Back up `vault.enc` before reinstalling, upgrading from the Tauri app, or switching how you unlock.

## Updating

Settings > Check for updates installs new versions on Windows, macOS, and the Linux AppImage. Installing locks the vault, verifies the download, installs, and closes Latch; open it again afterwards. For the deb, download and install the newer deb.

### Upgrading from the Tauri app

Versions before 0.3.1 were a Tauri app and won't offer this update. Install the native app by hand:

1. If your vault uses Google sign-in, unlock it in the old app first and choose Settings > Switch to master password. Google sign-in is retired, and the native app leaves unmigrated Google vaults untouched but can't open them. [Version 0.2.6](https://github.com/PeeeBrain/latch/releases/tag/v0.2.6) is still available for this.
2. Quit the old app fully, including its tray icon, and back up `vault.enc`.
3. On Windows, uninstall the old app (especially if you used the MSI). On macOS, replace the old Latch.app in Applications.
4. Install the native app as described in [Install](#install). Keep your vault file and OS keychain entries.
5. Check that the new app unlocks your vault before you delete the old installation.

## Troubleshooting

- **Ctrl+Space does nothing.** Open Settings to see what your desktop granted. On Linux, follow [Ctrl+Space to show and hide Latch](#ctrlspace-to-show-and-hide-latch). On any platform, another app may already own Ctrl+Space; change Latch's key in your desktop's shortcut settings where possible, or open Latch from the app launcher.
- **"Another Latch process is using this vault."** Latch is already running with this vault, perhaps hidden or from another install. Close it first.
- **A Google vault won't open.** Migrate it in the old app first; see [Upgrading from the Tauri app](#upgrading-from-the-tauri-app).
- **A vault won't open at all.** Latch never replaces a damaged or unsupported vault with an empty one. Keep the file and a backup, and open an issue.

## Development

### Run from source

Install Rust with rustup. `rust-toolchain.toml` selects the right toolchain, rustfmt, and Clippy. You also need:

- Windows: the MSVC C++ build tools and Windows SDK.
- macOS: the Xcode command line tools.
- Linux: the GPUI and GTK development packages listed in the native job in [ci.yml](.github/workflows/ci.yml).

From the repository root:

```sh
cargo run -p latch-desktop --locked
```

This builds and launches a debug build. Restart the command after changing code; an old executable keeps running the old code.

By default this opens your real vault. For UI work, point debug builds at a throwaway vault with `LATCH_DEV_VAULT`, and create its folder first. A missing vault starts setup, and appearance settings live beside whichever vault is open.

```sh
mkdir -p /tmp/latch-dev
LATCH_DEV_VAULT=/tmp/latch-dev/vault.enc cargo run -p latch-desktop --locked
```

In PowerShell:

```powershell
$env:LATCH_DEV_VAULT = Join-Path $env:TEMP 'latch-dev/vault.enc'
New-Item -ItemType Directory -Force (Split-Path $env:LATCH_DEV_VAULT) | Out-Null
cargo run -p latch-desktop --locked
Remove-Item Env:LATCH_DEV_VAULT  # back to your normal vault
```

On Linux, Ctrl+Space in a debug build needs the same desktop file as [Linux setup](#ctrlspace-to-show-and-hide-latch), with `Exec=` set to the full path of `target/debug/latch-desktop`.

### Checks

From the repository root:

```sh
cargo fmt --all -- --check
cargo check -p latch-desktop --all-targets --locked
cargo test -p latch-core --locked
cargo test -p latch-core --features legacy-oauth --locked
cargo test -p latch-desktop --locked
cargo clippy -p latch-core -p latch-desktop --all-targets --locked -- -D warnings
```

The `legacy-oauth` feature exists only to test that old vaults still open. The desktop app must never enable it.

### Code layout

- `crates/latch-core` owns everything that touches secrets: authentication, encryption, vault storage, the unlocked session, search, password generation, aliases, and health checks. It has no UI.
- `crates/latch-desktop` is the GPUI app: the window, text input, clipboard, tray, global shortcut, device-key integration, and signed updater.

The root Cargo workspace owns the toolchain, lockfile, and `target/` directory. [AGENTS.md](AGENTS.md) covers repository policy, [CONTRIBUTING.md](CONTRIBUTING.md) contribution guidance, and `docs/adr/` the main design decisions.

### CI builds

Pull-request CI uploads a release-mode executable for each platform as `gpui-preview-<OS>-<architecture>`. Extract it and run `latch-desktop.exe` (Windows) or `latch-desktop` (macOS/Linux; run `chmod +x` on it first). Previews use your normal vault location and are not installers.

The Native GPUI packages workflow builds the real installers. Maintainers can run it on a branch with `sign` enabled to rehearse a release without publishing; see [the release procedure](docs/releases.md).

## License

MIT. See [LICENSE](LICENSE).
