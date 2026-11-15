# Credentials

The user adds a Credential from a form (Title, Username, Password, Website, Authenticator), saves it with Enter, Ctrl+S, or the Save button, edits it from Browse or its details, and deletes it after a confirmation. Saved Credentials survive lock and unlock and never appear in plaintext in `vault.enc`.

## Sub-features

- `cred-new` opens an empty form and saves a new Credential.
- `cred-required` refuses a save with an empty Password and keeps the draft.
- `cred-edit` changes an existing Credential.
- `cred-details` shows a Credential with edit and delete.
- `cred-delete` removes a Credential after confirmation.
- `cred-persist` keeps changes across lock and unlock, encrypted at rest.

## How to get to it (user POV)

- New: Ctrl+N, or `New credential` in Ctrl+K or `>new`.
- Save: Enter in a text field, Ctrl+S anywhere in the form, Tab to `Save credential` and Enter, or click it.
- Edit: select a row, Ctrl+Shift+E; or details, `Edit`.
- Details: select a row, Ctrl+K, `View selected credential`, Enter.
- Delete: details, `Delete`, then confirm.

## Driving it with verify.sh

```text
unlock
press ctrl-n
expect focus field 0
shot new-form
type Example
press tab
type person@example.com
press tab tab
type first secret
press tab tab
type zara.invalid
click save-credential
expect rows Example
expect notice Saved to encrypted vault
shot saved
# Edit through the shortcut.
press ctrl-shift-e
expect page Edit
press ctrl-a
type Renamed
press tab tab tab ctrl-a backspace
press ctrl-s
expect page Edit
expect focus field 2
expect notice Password is required
shot required
type second secret
press ctrl-s
expect rows Renamed
# Persistence through a second view.
press ctrl-l
unlock
expect rows Renamed
expect vault-lacks second secret
# Details and delete.
press ctrl-k
type View selected credential
press enter
expect page Details
shot details
click detail-delete
expect page Delete
shot delete-confirm
click confirm
expect page Browse
expect rows
```

## Gotchas

- Form focus order: Title (field 0), Username (field 1), Alias button, Password (field 2), Generate button, Website (field 3), Authenticator (field 4). Re-derive tab counts from `views.rs` if a stop is added.
- Clicking a button moves focus to it; `type` after a `click` goes nowhere until a field has focus again.
- Enter on the Generate button generates instead of saving. Ctrl+S saves from anywhere in the form.
- Enter on a Delete or Discard confirmation hits `Cancel`, which has focus first. Click `confirm`, or press `right enter`.
- Websites are normalized on save (`zara.invalid` becomes `https://zara.invalid/`).
