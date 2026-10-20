# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Update GPUI Kit to 0.7.3 and refresh native dependencies.
- Update the JWT library used by historical Vault compatibility tests and select its crypto backend explicitly.

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
