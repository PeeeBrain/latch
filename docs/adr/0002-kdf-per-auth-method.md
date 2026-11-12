# KDF per AuthMethod

Latch uses a different key derivation function for each AuthMethod, matched to its threat model:

- **Password**: PBKDF2-HMAC-SHA256 (100,000 iterations) — well-established, FIPS-compliant, sufficient for a user-chosen master password.
- **OAuth**: Argon2id (64 MiB memory, 3 iterations, 4 parallelism) — memory-hard KDF that raises the cost of brute-forcing the Google user_id against the app secret.
- **Biometric**: OS keychain (Windows Credential Manager / macOS Keychain / Linux Secret Service, see ADR 0003) — the OS stores and retrieves the raw key; no derivation needed.

A single KDF for all three would be simpler but would either waste resources (Argon2id for every password unlock) or weaken security (PBKDF2 for an OAuth-derived key where the input space is much smaller).
