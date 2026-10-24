# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.4] - 2026-10-24

### Changed
- On Windows, minimizing or closing Latch hides it to the tray instead of leaving a taskbar button. Open it again from the tray, Ctrl+Space, or by launching Latch again. Quit from the tray; Ctrl+Q now locks and hides Latch there.
- Quieter screens: unlock, setup, empty, and confirmation pages drop their large icon tiles, and a clean health check reads as plain text instead of a green badge.
- Edit page messages appear once, below the form in neutral text, instead of also in the footer and in amber for every message.

### Fixed
- Placeholder text and dark-mode primary button hover in the default colors now meet WCAG AA contrast.
- Update TLS certificate verification (rustls-webpki, webpki-roots) so update checks and favicons no longer trust distrusted intermediate certificates (RUSTSEC-2026-0381, RUSTSEC-2026-0382).

## [0.3.3] - 2026-10-20

### Changed
- Edit selected credential is now Ctrl/Cmd+Shift+E, freeing Ctrl/Cmd+E for global shortcuts.

### Fixed
- Ctrl+Left/Right (Option on macOS) jump by word in text fields, and add Shift to select. Password fields jump to the ends.
- The first Down in the actions menu moves to the next action instead of the one already highlighted.
- Keyboard focus returns to Latch after importing, applying, or leaving an appearance.
- Keep primary button labels and shortcuts legible across themes, using contrasting text for normal and hover backgrounds when the theme's button text color is too faint.
- Accept website domains such as `zara.com` when creating or editing credentials, adding HTTPS automatically so an existing website cannot block a title-only edit.
- Unfinished text composition in a hidden search field no longer silently blocks saving credentials. Unfinished form input now shows a message instead of ignoring Save.
- Give new and existing credentials a persistent Save credential button and Ctrl/Cmd+S shortcut, including when Alias or Generate has keyboard focus.
- Show credential save errors in full inside the form so they can be corrected without discarding changes.

## [0.3.2] - 2026-10-20

### Changed
- Update GPUI Kit to 0.7.3 and refresh native dependencies.
- Update the JWT library used by historical Vault compatibility tests and select its crypto backend explicitly.

### Fixed
- Rows, the back button, and footer links now show feedback the moment they are pressed.
- Credential initials stay legible on light themes.
- The credential details page no longer repaints every second unless a one-time code is counting down.

## [0.3.1] - 2026-10-18

### Changed
- Restore the original Latch logo for native app packages, Windows executables, and the tray.
- Move native CI and release packaging to Blacksmith runners.
- Support Apple Silicon macOS, Windows x64, and Linux x64; drop Intel macOS builds.
- Generate release changelogs from merged pull requests and preserve installation guidance.
- Remove the OpenCode workflow.

### Native release preparation
- Release 0.3.1 as the native GPUI release for Windows, macOS, and Linux.
- Replace Tauri release builds with tag-triggered native packaging, verified signatures, checksums, and a separate native updater feed.
- Require manual installation when moving from Tauri; migrate Google vaults in the previous app first.
- Remove the retired Tauri/React frontend, Bun tooling, OAuth configuration, and obsolete CI jobs.
- Keep Linux deb upgrades manual and restrict in-app updates to AppImage launches.

### Added
- Standard repository documentation (LICENSE, CONTRIBUTING.md, issue templates)
- Security audit improvements with cargo-binstall and frontend audit

### Changed
- Bumped version to 0.2.0

### Fixed
- Clipboard history retention for copied passwords
- Resolved console warnings across the frontend components
