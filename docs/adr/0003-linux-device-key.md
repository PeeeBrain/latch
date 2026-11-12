# Linux device key: polkit gate over the login keyring

Linux device-key Vaults use the same `biometric-keychain` header and raw 32-byte key as Windows and macOS. Only key custody differs:

- **Gate**: Latch asks polkit to authorize `com.latch.vault.unlock` (`auth_self`, no `_keep`). polkit runs the user's PAM stack, so with fingerprint login enabled (`pam_fprintd`) the desktop's polkit agent asks for a fingerprint and falls back to the login password.
- **Storage**: the key lives in the login keyring through the Secret Service (`oo7`), under `service=com.latch.vault`, `account=vault-encryption-key`. Latch reads it only after polkit authorizes.

## Why not fprintd directly

fprintd answers "did a finger match", with no fallback. A device-key Vault has no master password, so a dead reader, a removed fprintd, or re-enrolled fingers would lose the Vault. Windows Hello falls back to a PIN and Touch ID to the device password; polkit's PAM fallback to the login password is the Linux equivalent.

## Trust model

This is weaker than Windows and macOS. Windows wraps the key with a Hello-held key; macOS's Keychain enforces the user-presence ACL. On Linux the polkit check is enforced in Latch's process, and while the login keyring is unlocked any process running as the user can read the key without a fingerprint.

So the fingerprint protects against someone at an unlocked machine, not against malware running as the user, which on Linux could also capture a typed master password. Linux has no OS store that releases a secret only after a fingerprint; a TPM cannot be bound to fprintd either.

## Consequences

- The deb installs the policy to `/usr/share/polkit-1/actions/`. AppImage users install it by hand; without it Latch hides "Switch to device key" and explains why a device-key Vault can't unlock.
- Resetting the login keyring (for example, an admin resetting the login password) loses the key and the Vault, like a TPM reset on Windows.
- Needs a Secret Service provider (GNOME Keyring, KWallet) and a polkit agent, which every mainstream desktop ships.
