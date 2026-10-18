# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Native release preparation
- Prepare 0.3.0 as the native GPUI release for Windows, macOS, and Linux.
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
