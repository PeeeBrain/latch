# Latch Password Manager

A desktop password manager with a Raycast-style command palette UI. Credentials are stored in an encrypted Vault on disk, decrypted into an in-memory Workspace during an active Session.

## Language

**Vault**:
The encrypted file on disk (`vault.enc`) containing authentication metadata and ciphertext.
_Avoid_: store, database, file

**Workspace**:
The in-memory decrypted state — a list of Credentials and the Session key. Exists only during an active Session.
_Avoid_: vault (distinct from Vault), state, cache

**Credential**:
A single stored login: id, title, username, password, url, and icon_url.
_Avoid_: entry, record, item

**AuthMethod**:
How a Vault was provisioned and must be accessed: **Password** (master password + PBKDF2) or **Biometric** (device key in the OS keychain).
_Avoid_: KDF, auth type, login method

**Provision**:
Create a new Vault from scratch. Requires an AuthMethod and its credentials.
_Avoid_: init, setup, create

**Access**:
Unlock an existing Vault using the correct AuthMethod. Produces a Workspace and starts a Session.
_Avoid_: unlock, open, login

**Rotate**:
Re-encrypt a Vault with a new key, optionally changing the AuthMethod. The Workspace survives the rotation.
_Avoid_: re-encrypt, migrate, change password

**Session**:
A time-limited period (30 minutes) where the Workspace is available. Auto-locks on expiry.
_Avoid_: login session, unlock period

**Lockout**:
Exponential backoff after failed Access attempts. Base 5 seconds, doubles each failure, max 5 minutes. Resets on successful Access or app restart.
_Avoid_: rate limit, throttle

**VaultHealth**:
Unified analysis run against all Credentials: weak passwords (zxcvbn score < 3), reused passwords (same password across multiple Credentials), and breached credentials (Pwned Passwords API).
_Avoid_: audit, security check

**ClipboardGuard**:
Copies a password to the system clipboard and clears it after 30 seconds.
_Avoid_: clipboard timeout, clipboard watcher

**Command Palette**:
The single-window Raycast-style interface. Presents a search bar, results list, and page-specific panels.
_Avoid_: main window, UI, frontend

**Page**:
A discrete screen within the Command Palette (`Page` enum in `latch-desktop/src/app.rs`): Setup, Locked, Browse, Details, Edit, Delete, Settings, Rotate, Health, Actions, Appearance, Preview, Providers, Provider, DeleteProvider, Generator, Discard.
_Avoid_: mode, view, screen

## Relationships

- A **Vault** is **Provisioned** once, then **Accessed** repeatedly.
- An **Access** produces a **Workspace** bound to a **Session**.
- A **Workspace** contains zero or more **Credentials**.
- **Rotate** changes a **Vault**'s encryption without affecting the **Workspace**.
- Failed **Access** attempts trigger a **Lockout**.
- **VaultHealth** audits all **Credentials** in the **Workspace**.
- **ClipboardGuard** protects a single **Credential** password after copying.
- The **Command Palette** navigates between **Pages** to expose different operations.

## Example dialogue

> **Dev:** "When a user opens the app, do we Provision or Access?"
> **Domain expert:** "If no Vault exists, we Provision one via the selected AuthMethod. If a Vault already exists, we Access it — the user provides their credentials, we derive the key, and open a Workspace."
>
> **Dev:** "What happens on a failed Access attempt?"
> **Domain expert:** "The Lockout kicks in. First failure is a 5-second wait. Each subsequent failure doubles the wait, up to 5 minutes. A successful Access resets the counter."
>
> **Dev:** "Does Rotate change the Credentials?"
> **Domain expert:** "No — Rotate only changes the encryption layer. The Workspace is decrypted with the old key, re-encrypted with the new one, and written back to the Vault. Credentials are untouched."

## Flagged ambiguities

- "vault" was used to mean both the encrypted file on disk and the in-memory decrypted state — resolved: **Vault** (disk) and **Workspace** (memory) are distinct concepts.
- "entry" was the code term for a stored credential — resolved: renamed to **Credential**.
- KDF strings such as "password-pbkdf2" were used as auth method identifiers — resolved: **AuthMethod** is a domain enum; KDF is an implementation detail (see ADR-0002).
