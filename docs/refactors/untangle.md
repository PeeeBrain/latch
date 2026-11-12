# Untangle decisions

This pass preserves behavior, Vault bytes and paths, device-key identifiers, and Session timing. The audit covers both crates, release scripts, workflows, and their documentation. Native Luna subagents with high reasoning performed the area audits. Both optional comment and test cleanup were requested.

## Changes

- Put Workspace persistence with the Workspace, rather than Credential CRUD.
  - Alias configuration writes import `entries::persist`, even though they do not edit Credentials.
  - Credential and alias writes must serialize the same complete Workspace and preserve auth metadata.
- Keep Session activity validation together in Workspace.
  - Adjacent `check_session` and `refresh` calls recur in search, CRUD, alias operations, and desktop health/activity paths.
  - Preserve the delayed refresh in Credential creation and validation-only checks in Rotate.
- Analyze password strength once per Credential for a full health report.
  - Weak-password reporting and average entropy currently repeat the same expensive analysis.
- Let the HIBP checker own its HTTP client across Credential checks.
  - Constructing a client per password prevents connection reuse during a report.
  - Preserve timeout, redirect prohibition, request headers, response limits, and optional network access.
- Parse Appearance JSONC once with one grammar.
  - `Appearance::parse` and source inference duplicate parser options and depth checks.
  - Loading a managed Appearance parses the same text twice.
  - Stop writing the unused `appearance-source` marker; source inference uses the settings themselves.
- Declare shortcut chords alongside their actions and reuse them for command hints and key bindings.
  - `COMMANDS` hints and `init` bindings independently describe the same shortcuts.
  - Preserve both Ctrl and Cmd bindings on every platform.
- Type the commands passed from desktop integration to the app.
  - Tray and hotkey producers pass strings that the app silently ignores when unknown.
  - Keep menu IDs at the external event boundary.
- Remove redundant configuration.
  - `tempfile` is both a normal and dev dependency, though the normal dependency serves both.
  - CI packaging filters name two configuration files that do not exist.
- Improve legacy OAuth regression tests and remove restatement comments.
  - Tests named valid-token tests currently reject malformed fixtures before exercising their intended behavior.
  - Fixed-size key length assertions cannot detect incorrect derivation.
  - Keep independent historical-writer compatibility fixtures.

## Decisions to preserve

- `with_vault` explicitly routes UI operations through VaultCoordinator. Replacing it with a large forwarding API would add machinery; core operation functions remain responsible for their session requirements.
- Public compatibility types and AuthMethod tags stay stable, including the unused public `all_tags` helper. Lack of a repository caller alone does not establish that a public API should disappear.
- Keep historical KDF readers, serde defaults, TOTP preserve/clear semantics, fixed supported TOTP parameters, capped Lockout, Session generations and clock rollback handling, alias cancellation, atomic writes, and Rotate verification. These encode compatibility or safety requirements.
- Keep provider-specific network protocols, Windows prompt handling, Linux tray thread and visibility policy, and the ignored public-network favicon test. They reflect external constraints.
- The app state/render split remains. Moving large blocks without reducing dependencies would spread private-state coupling across more files.
- Release platform lists span workflow matrices, artifact validation, tests, and installation docs. These are separate consumers of the supported release targets; no new YAML parser or generator is justified for three stable targets.
- Release tests cover distinct artifact/signature/tag failure modes and stay intact. Desktop tests exercise keyboard, persistence, and lock behavior and stay intact.
- The contribution checklist includes legacy-feature validation beyond AGENTS.md's checklist. Run those additional checks for this pass; avoid changing agent instructions as part of the runtime refactor.

## Verification

- Before edits: all 87 core tests passed.
- After edits: all 88 default core tests and all 104 legacy-feature core tests passed.
- Desktop all-target check and Clippy with warnings denied passed. Core all-feature Clippy with warnings denied passed.
- Independent Luna/high review checked persistence ordering, rollback, auth metadata, Session timing, health results, and HIBP request restrictions. No remaining worthwhile core findings.
- Independent Luna/high desktop review found four additional shortcut hints with duplicated chords. These now use the shared declaration; re-review found no remaining worthwhile desktop findings.
- OAuth fixture validation initially failed because the tests supplied an audience without a configured client ID. The fixtures now omit that claim when unconfigured; production authentication was not changed.
- Desktop tests passed: 20 passed, one existing public-network favicon test ignored. Final formatting and whitespace checks passed.
- No app launch or native platform interaction was attempted, per the repository policy. Keyboard shortcuts, tray commands, and Appearance loading should receive a native smoke check in a CI artifact. Release scripts were unchanged; release-tooling and workflow checks remain with CI.
