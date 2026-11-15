# Command Palette

Ctrl+K opens the actions list over the current Page with its filter focused; typing filters and Enter runs the first match. On Browse, a query starting with `>` lists root commands instead of Credentials. Escape goes back. Leaving a form with unsaved text asks to discard it first.

## Sub-features

- `actions-open` opens the actions list with its filter focused.
- `actions-run` runs the first matching action on Enter.
- `root-commands` lists commands for a `>` query and runs one on Enter.
- `back` returns to the previous Page on Escape.
- `discard-guard` asks before leaving a dirty form.
- `form-guard` keeps Generator, Rotate, and Provider until Escape.

## How to get to it (user POV)

- Ctrl+K from any unlocked Page.
- `>` then a command name in the Browse search field.
- Escape from any Page, form, or confirmation.

## Driving it with verify.sh

```text
unlock
add Example | person@example.com | secret | example.invalid
press ctrl-k
expect page Actions
expect focus actions
shot actions
type generate
shot actions-filtered
press enter
expect page Generator
# Escape walks back through Actions.
press escape
expect page Actions
press escape
expect page Browse
press ctrl-a
type >health
shot root-commands
press enter
expect page Health
press escape
press ctrl-n
type Draft
press escape
expect page Discard
shot discard
press enter
expect page Edit
expect focus field 0
press escape
press right enter
expect page Browse
```

## Gotchas

- `View`, `Edit`, `Copy username`, and `Copy one-time code` need a selected Credential; with none, Enter does nothing visible.
- Enter on a confirmation hits `Cancel` first; `right enter` confirms.
- From Generator, Ctrl+N is refused with `Finish this form or press Escape before switching pages.`
