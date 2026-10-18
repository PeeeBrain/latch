# Native releases

The release version comes from `crates/latch-desktop/Cargo.toml`. Keep `latch-core` at the same version. Only stable `MAJOR.MINOR.PATCH` versions are accepted.

## Before tagging

1. Update both crate versions, `Cargo.lock`, `CHANGELOG.md`, and `docs/release-notes.md` in a PR. Run the checks in AGENTS.md.
2. Dispatch Native GPUI packages on the PR branch with `sign` enabled. It produces signed release downloads without publishing anything. Verify the installers and vault access on the supported platforms.
3. Merge the PR into `main`. A Cargo version change alone never starts a release.
4. Tag the merged commit and push that tag:

   ```sh
   git switch main
   git pull --ff-only
   git tag -a v0.3.0 -m "Latch 0.3.0"
   git push origin v0.3.0
   ```

The workflow checks that the tag matches the Cargo version, resolves to the checkout, and is reachable from `origin/main`. Do not move published tags. The workflow never creates version commits or tags automatically. Maintainers can retry an existing tag with Release > Run workflow, using the same tag as input.

## Build and publication

The shared native package workflow tests and builds Windows x64 NSIS, macOS ARM64 app/DMG, and Linux x64 deb/AppImage packages. All builds use the root lockfile and pinned Cargo Packager 0.11.8. Linux packages use Ubuntu 24.04 as their baseline.

The existing `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repository secrets remain in use to preserve the updater trust key. The key secret has the same outer base64 encoding that the previous release workflow decoded into a signing-key file. The packager reads that file through `CARGO_PACKAGER_SIGN_PRIVATE_KEY`. Temporary key files are removed after packaging. Do not print or commit private keys.

Publication waits for every target. The release script requires every download and signature, verifies all signatures with Minisign against `crates/latch-desktop/updater.pub`, and creates `SHA256SUMS` and `native-latest.json`. A missing asset, missing signature, wrong key, or changed payload stops publication. Asset names include version, OS, and architecture.

The workflow generates release notes through GitHub's API from the previous reachable stable `vMAJOR.MINOR.PATCH` tag, listing merged pull requests, authors, and a full changelog comparison link. Prereleases and unrelated or malformed tags are ignored. If there is no previous stable tag, GitHub generates notes from the full history. Installation and migration instructions from `docs/release-notes.md` follow the generated changelog.

The workflow uploads the complete set to a draft release, then publishes it as latest. Failed draft uploads can be retried. Published releases are never overwritten. Only one release version should be in flight at a time.

Native updates use the `windows-x86_64`, `macos-aarch64`, and `linux-x86_64` target keys, with `nsis`, `app`, and `appimage` formats. Deb and DMG remain manual downloads. The application checks the HTTPS GitHub host and verifies signatures before installation.

## Installation and migration

Follow [README.md](../README.md#downloads-and-upgrades). Tauri-to-GPUI installer continuity has not been verified, so the native packages are not published to the retired `latest.json` feed. Existing users must download and install the native release manually. Google vaults must first be switched to a master password in the previous app. Version 0.2.6 and its source tag remain available for migration.

Updater signatures do not provide Windows Authenticode or Apple Developer ID signing/notarization. These OS credentials are not configured in this repository. Expect unknown-publisher/Gatekeeper prompts. Native app launch, device-key access, GPU behavior, accessibility, and an installed native-to-native update still need acceptance on real supported platforms before announcing a production release.

## Local validation

Use Python 3.11+ and Minisign on PATH:

```sh
python -m unittest discover -s scripts -p 'test_*.py' -v
```

To verify all downloaded signed rehearsal artifacts together without publishing:

```sh
python scripts/release.py manifest <artifact-directory> --repository PeeeBrain/latch
```

Builds and packaging run only on GitHub Actions, per the no-local-build policy. `act` can list workflows locally, but do not execute build jobs through it.
