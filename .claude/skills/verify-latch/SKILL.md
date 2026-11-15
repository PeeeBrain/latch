---
name: verify-latch
description: See and prove a Latch UI change. Plays a plain-text script against the real Command Palette in a sandbox Vault, renders real screenshots, and compares before and after. No app build or launch, no Rust to write. Use to verify a latch-desktop change, check how a screen looks, reproduce a UI bug, or walk a user journey (setup, unlock, credentials, search, copy, palette, generator).
---

# Verify Latch

`verify.sh` plays a **script** against the real `Latch` view: the same Pages, keymaps, focus, and Vault code as the app, rendered by the platform's fonts and GPU renderer into PNG **screenshots**. It runs as `cargo test`, so it obeys AGENTS.md (no build or launch), and every run gets a fresh **sandbox Vault**, so you can create, edit, reveal, and delete anything. Your real Vault is never touched.

All commands run from the repo root. `V=.agents/skills/verify-latch/verify.sh`.

## 1. Doctor

`$V doctor`. Done when it ends `check: ok`. A failure is a broken base: report it before driving.

## 2. Write a script

A text file, one command per line, `#` for comments. Start from the matching recipe in [`features/`](features/README.md).

| Command | Does |
| --- | --- |
| `unlock` | Types the sandbox master password and presses Enter (on first run, fills Setup's two fields). Start every script with it. |
| `add Title \| user \| password [\| website [\| totp]]` | Creates a Credential through the real New credential form. Use `.invalid` websites. |
| `type <text>` | Types into whatever has focus. |
| `press <keys>` | Space-separated keys: `enter`, `escape`, `tab`, `shift-tab`, `down`, `ctrl-k`, `ctrl-shift-e`. |
| `click <element-id>` | Clicks an element on screen (nothing scrolls; use a taller size for rows below the fold), e.g. `save-credential`, `detail-edit`, `detail-delete`, `confirm`, `cancel`, `use-generated`, `copy-generated`, `reveal`. Ids live in `crates/latch-desktop/src/app/views.rs`. |
| `shot <name>` | Saves a screenshot. |
| `expect page <Page>` | `Setup`, `Locked`, `Browse`, `Details`, `Edit`, `Delete`, `Settings`, `Health`, `Actions`, `Generator`, `Discard`, … |
| `expect focus <name>` | `search`, `actions`, `field N` (form fields from 0), `stop N` (buttons and rows), `none`. |
| `expect rows <A, B>` | Credential rows, comma-separated; nothing after `rows` means empty. |
| `expect notice <text>` | The footer notice contains the text. |
| `expect clipboard <text>` | Latch's clipboard holds exactly the text; nothing after means empty. |
| `expect visible <element-id>` | The element is laid out and visible. |
| `expect vault-lacks <text>` | `vault.enc` does not contain the text in plaintext. |
| `sleep <ms>` | Real time, for Lockout backoff. |

Done when every behavior you claim has an `expect`, and every screen you claim has a `shot`.

## 3. Run it

- `$V drive script.txt [WxH]` drives the working tree. Default size `720x480`; the minimum window is `520x360`.
- `$V before script.txt [rev] [WxH]` runs the same script on `rev` (default: merge-base with `main`) and on the working tree, writes `NN-name-before-after.png` side by side, and prints `pixels changed` per screenshot. Use it for every visual change.

After each command the transcript prints `page= focus= rows= notice=`. A failed command or `expect` stops the run, names the line, prints the state, and saves `NN-failure.png`. Then `Read` the PNGs and look: layout, clipping, alignment, focus ring, notice text.

`before` builds the base in its own target dir; the first time takes about 2 minutes, later runs about 15 seconds. `drive` takes about 10 seconds.

## 4. Evidence

Each run writes `target/verify/<script name>/<UTC time>/`: `script.txt`, `transcript.log`, the screenshots (under `before/` and `after/` for a comparison), and the sandbox `vault/`. Report the directory, the screenshots you looked at, what they show, and the `pixels changed` counts. Rendering is deterministic, so `0` means that screen did not change.

The sandbox renders the real view, not the OS window. These stay unverified and need the developer's `cargo run` with `LATCH_DEV_VAULT`, or a CI artifact. Name them in your report when your change touches them:

- Dark mode: screenshots are always the light appearance.
- Window controls (minimize, maximize, close): drawn only under client-side decorations, so they are absent here.
- Global Ctrl/Cmd+Space hotkey, tray, show/hide, Wayland activation.
- The OS clipboard and its 30-second clear: `expect clipboard` sees Latch's test clipboard.
- Device key (Windows Hello, Keychain, polkit + Secret Service), IME, and network (favicons, HIBP, aliases, updater).

## 5. Cleanup

Nothing to undo: the driver is a permanent ignored test (`crates/latch-desktop/src/app/drive.rs`), and sandbox Vaults live in the evidence directory. `before` keeps its base checkout in `target/verify/worktrees/<rev>` for speed; remove one with `git worktree remove --force target/verify/worktrees/<rev>`.

To reuse one sandbox across runs, set `VAULT=target/verify/sandbox/vault.enc`; its master password is the driver's `SANDBOX_PASSWORD`.

## Feature map

[`features/`](features/README.md) holds a script recipe per feature. When a change adds or moves a user path, update its recipe in the same change.
