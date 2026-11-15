# Search and copy

Browse filters Credentials as the user types. Enter copies the selected Credential's password, Ctrl+Shift+C its username, and Ctrl+T its current one-time code. Each copy shows `Copied. Clipboard clears in 30 seconds.` Locking clears the clipboard, but only while Latch still owns it.

## Sub-features

- `search-filter` narrows rows by the query.
- `search-select` moves the selection with Up and Down.
- `copy-password` copies the password on Enter.
- `copy-username` copies the username.
- `copy-totp` copies a one-time code, or reports `No TOTP configured`.
- `clipboard-lock` clears Latch's copy on lock.

## How to get to it (user POV)

- Browse is the page after unlock; the search field has focus.
- Enter on a row, Ctrl+Shift+C, Ctrl+T, or `Copy username` / `Copy one-time code` in Ctrl+K.
- From details, Enter copies the password.

## Driving it with verify.sh

```text
unlock
add Alpha | alpha@example.com | Alpha secret | alpha.invalid
add Bravo | bravo@example.com | Bravo secret | bravo.invalid | JBSWY3DPEHPK3PXP
shot two-rows
type bra
expect rows Bravo
shot filtered
press enter
expect notice Copied
expect clipboard Bravo secret
press ctrl-shift-c
expect clipboard bravo@example.com
press ctrl-t
expect notice Copied
press ctrl-a
type alp
press ctrl-t
expect notice No TOTP configured
press ctrl-l
expect clipboard
```

## Gotchas

- A query starting with `>` lists commands only; no Credential row is selectable.
- `expect clipboard` sees Latch's test clipboard. The OS clipboard, the 30-second clear, and Wayland ownership need a real run.
- The one-time code changes every 30 s; assert the notice, not the digits.
