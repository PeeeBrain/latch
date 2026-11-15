# Latch verification map

One file per user-facing feature. Each holds a script recipe to copy, run with `verify.sh drive` or `verify.sh before`, and extend. `SKILL.md` lists every script command.

## Conventions

- Every script starts with `unlock`; the sandbox Vault is fresh, so the first `unlock` provisions it through Setup.
- Seed Credentials with `add`, which drives the real form. Websites use `.invalid` hosts so favicon fetches never leave the machine.
- Write Ctrl chords. Every shortcut is also bound to `cmd-`, and Ctrl exercises the same action on every OS.
- `shot` every screen whose look you are claiming, `expect` every behavior.

## Proof and skip reporting

- Report the evidence directory, the recipe IDs driven, and the entry point each used (keyboard, palette, pointer).
- A sub-feature proven through one entry point is verified for that entry point only. List the others as not driven.
- When a precondition cannot be reached, report the run as failed with the reason and add the reason to that feature's `Gotchas`. Keep the bar where it is.

## Feature entry contract

H1, one paragraph of user-visible behavior, then four H2s in order: `Sub-features` (short IDs), `How to get to it (user POV)` (every entry point), `Driving it with verify.sh` (a script recipe, then notes), and `Gotchas`.

## Features

Every recipe below passes as written.

- [Setup and unlock](./unlock.md): Provision, password rules, lock, Access, wrong password, Lockout.
- [Credentials](./credentials.md): new, edit, required fields, details, delete, persistence.
- [Search and copy](./search-copy.md): filtering, copy password, username, one-time code, clipboard on lock.
- [Command Palette](./palette.md): Ctrl+K actions, `>` root commands, Escape, the unsaved-draft guard.
- [Password generator](./generator.md): open, length, use, copy, discard. Run at `720x900`.
- [Settings](./settings.md): open, Start at login.

Not yet mapped: Password health, the rest of Settings, master password and device-key Rotate, Appearance import, alias providers.
