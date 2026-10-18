Latch now uses a native GPUI interface on Windows, macOS, and Linux.

## Install

- Windows x64: download and run `Latch_0.3.0_windows-x86_64-setup.exe`.
- macOS Apple Silicon: download `Latch_0.3.0_macos-aarch64.dmg` and copy Latch to Applications.
- Linux x64: download `Latch_0.3.0_linux-x86_64.AppImage`, give it executable permission, and launch it. Ubuntu 24.04 users can use `Latch_0.3.0_linux-x86_64.deb` instead.

Checksums are in `SHA256SUMS`. Updater `.sig` and `.app.tar.gz` files are for in-app updates. Packages carry updater signatures but are not Windows Authenticode signed or Apple Developer ID signed/notarized, so the OS may require explicit approval before opening them.

## Upgrade from the previous app

This release requires manual installation for Tauri users. Quit Latch fully, including its tray process, and back up `vault.enc` before changing installations. Keep the vault file and OS keychain entries. On Windows, uninstall the previous app before running the native installer. On macOS, replace the previous Latch.app in Applications.

Google OAuth has been retired. If you have a Google vault, unlock it in the previous app and use Settings > Switch to master password before upgrading. The native app preserves unmigrated Google vaults and cannot unlock them. [Version 0.2.6](https://github.com/PeeeBrain/latch/releases/tag/v0.2.6) remains available for migration. Verify that the new app can unlock your vault before discarding access to the previous installation.

Subsequent native releases use the signed native update feed through Settings > Check for updates. Linux in-app updates require launching the AppImage. Upgrade deb installations by downloading and installing the newer deb.
