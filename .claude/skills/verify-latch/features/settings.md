# Settings

Settings lists Security, Preferences, and About rows. This recipe covers `Start at login`, a check box row that registers Latch to open minimized when the user signs in.

## Sub-features

- `settings-open` shows Settings with its first row focused.
- `start-at-login` toggles the login entry and its check box.

## How to get to it (user POV)

- Ctrl+, from any unlocked Page, `Settings` in Ctrl+K, or `>settings` on Browse.
- On Settings: Tab to `Start at login` and press Enter, or click it.

## Driving it with verify.sh

```text
unlock
press ctrl-,
expect page Settings
expect focus stop 0
shot settings
click start-at-login
expect page Settings
shot start-at-login-on
click start-at-login
shot start-at-login-off
```

## Gotchas

- Under `cargo test` the login entry is kept in memory, so the drive never registers the test binary with the OS. The check box state is the proof; the OS entry needs the developer's `cargo run` or a CI artifact.
- Turning it on from `cargo run` registers `target/debug/latch-desktop`, which opens the real Vault at the next login (no `LATCH_DEV_VAULT` there). Turn it off again after checking.
