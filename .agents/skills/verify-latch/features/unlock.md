# Setup and unlock

On first run the user picks a master password twice to Provision the Vault, then lands on Browse. Locking drops the Workspace; the master password and Enter Access it again. A wrong password is refused without saying which part was wrong, and each failure starts an exponential Lockout.

## Sub-features

- `setup-reject` refuses a password outside 12 to 1024 bytes or a mismatched confirmation, and writes nothing.
- `setup-provision` creates the Vault and opens Browse.
- `lock` returns to Locked and clears rows.
- `unlock-password` Accesses the Vault with the master password.
- `unlock-wrong` refuses a wrong password with a neutral notice.
- `lockout` refuses even the right password until the backoff passes.

## How to get to it (user POV)

- First launch with no Vault shows Setup: Master password, Tab, Confirm master password, Enter.
- Ctrl+L, or `Lock vault` in Ctrl+K or a `>lock` query.
- The Locked page: type the master password, Enter.

## Driving it with verify.sh

```text
expect page Setup
shot setup
type too short
press tab
type too short
press enter
expect page Setup
expect notice Use 12 to 1024 bytes
shot setup-rejected
# A rejected setup keeps both drafts with focus on Confirm.
press ctrl-a backspace shift-tab ctrl-a backspace
unlock
expect page Browse
press ctrl-l
expect page Locked
shot locked
type not the master password
press enter
expect notice Incorrect password or key, or the vault is damaged.
shot wrong-password
unlock
expect notice Too many failed attempts
sleep 5100
unlock
expect page Browse
```

## Gotchas

- Lockout runs on the real clock from the first failure: 5 s, doubling, 5 min after 10 failures. The script must `sleep`.
- A failed Access clears the field; a rejected Setup keeps both drafts.
- Device-key unlock reaches the OS keyring and polkit, which the sandbox cannot.
- A wrong key and a damaged Vault share one notice by design. Assert that wording.
