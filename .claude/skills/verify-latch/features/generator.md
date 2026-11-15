# Password generator

The generator shows a fresh password, lets the user change its length and options, and either copies or uses it. Using it from Browse opens a new Credential with the Password filled; using it from an open form fills that form and returns to it.

## Sub-features

- `gen-open` shows a generated password.
- `gen-length` shortens or lengthens it, clamped to 8–128.
- `gen-use-new` opens a new Credential with the password filled.
- `gen-use-edit` fills the Password of the form it came from.
- `gen-copy` copies the generated password.
- `gen-discard` leaves a generated draft without saving.

## How to get to it (user POV)

- `Generate password` in Ctrl+K, or `>generate` on Browse.
- The `Generate` button beside Password in the Credential form.
- On the generator: Left/Right or `Shorter`/`Longer`, Enter or `Use password`, `Copy`.

## Driving it with verify.sh

```text
# Run at 720x900: Regenerate and Copy sit below the fold at the default size.
unlock
press ctrl-k
type generate
press enter
expect page Generator
shot generator
press left left left
shot shorter
click copy-generated
expect notice Copied
# Focus is on Copy now, so Enter would copy again.
click use-generated
expect page Edit
expect focus field 0
shot use-new
press escape
expect page Discard
press right enter
expect page Browse
expect rows
# From an open form.
press ctrl-n
type Example
press tab tab tab tab
press enter
expect page Generator
press enter
expect page Edit
shot use-edit
```

## Gotchas

- At 720x480 only the options and `Use password` fit; `regenerate` and `copy-generated` need a taller size to `click`.
- Generator owns Left/Right; elsewhere they move between confirmation buttons.
- Ctrl+N from the generator is refused with `Finish this form or press Escape before switching pages.`
- Enter on the Generate button generates; Ctrl+S still saves the form.
