use super::{storage::VaultStorage, workspace::Workspace, EncryptedVault, VaultData};
use crate::auth::method::AuthMethod;
use crate::crypto::aead;

pub fn rotate(
    storage: &VaultStorage,
    workspace: &mut Workspace,
    new_key: &[u8; 32],
    new_method: AuthMethod,
    new_salt: &str,
) -> Result<(), String> {
    workspace.check_session()?;

    let vault_data = VaultData::from_workspace(workspace);
    let json = zeroize::Zeroizing::new(
        serde_json::to_string(&vault_data)
            .map_err(|e| format!("Failed to serialize vault data: {}", e))?,
    );
    let encrypted = aead::encrypt(new_key, &json)?;

    let vault = EncryptedVault {
        version: "2".to_string(),
        kdf: new_method.vault_tag().to_string(),
        salt: new_salt.to_string(),
        data: encrypted,
    };

    // Verify the replacement ciphertext before the atomic commit. The session
    // changes only after that commit, so failed writes preserve the old access.
    let verified = zeroize::Zeroizing::new(aead::decrypt(new_key, &vault.data)?);
    if verified.as_str() != json.as_str() {
        return Err("Replacement vault verification failed".into());
    }
    storage.write(&vault)?;
    workspace.start(*new_key);

    Ok(())
}

pub fn password(
    storage: &VaultStorage,
    workspace: &mut Workspace,
    password: &str,
) -> Result<(), String> {
    if password.len() < 12 || password.len() > 1024 {
        return Err("Master password must contain 12 to 1024 bytes".into());
    }
    let salt = crate::auth::password::generate_salt();
    let key = zeroize::Zeroizing::new(crate::auth::password::derive_key(password, &salt));
    rotate(
        storage,
        workspace,
        &key,
        AuthMethod::Password,
        &hex::encode(salt),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::authenticator::{AuthCredential, Authenticator};
    use crate::vault::{AliasConfig, Entry};

    #[cfg(feature = "legacy-oauth")]
    #[test]
    fn original_oauth_writers_can_migrate_to_a_reopenable_password_vault() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let user_id = "historical-fixture-user";
        let mut claims =
            serde_json::json!({"sub":user_id,"iss":"accounts.google.com","exp":4_102_444_800_u64});
        if let Some(client_id) = std::env::var("LATCH_OAUTH_CLIENT_ID")
            .ok()
            .filter(|value| !value.is_empty())
        {
            claims["aud"] = client_id.into();
        }
        let token = format!(
            "{}.{}.signature",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256"}"#),
            URL_SAFE_NO_PAD.encode(claims.to_string())
        );
        for tag in ["oauth-pbkdf2", "oauth-argon2id"] {
            let directory = tempfile::tempdir().unwrap();
            let storage = VaultStorage {
                path: directory.path().join("vault.enc"),
            };
            // Reconstruct each published writer independently of the compatibility reader.
            let fallback = if tag == "oauth-pbkdf2" {
                "latch-dev-secret-32bytes-long!!"
            } else {
                "test-secret-for-development-only-32b"
            };
            let secret = zeroize::Zeroizing::new(
                std::env::var("LATCH_OAUTH_SECRET").unwrap_or_else(|_| fallback.into()),
            );
            let salt = format!("latch-vault-oauth-{user_id}");
            let mut writer_key = [0u8; 32];
            if tag == "oauth-pbkdf2" {
                pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
                    secret.as_bytes(),
                    salt.as_bytes(),
                    100_000,
                    &mut writer_key,
                );
            } else {
                let parameters = argon2::Params::new(65536, 3, 4, Some(32)).unwrap();
                argon2::Argon2::new(
                    argon2::Algorithm::Argon2id,
                    argon2::Version::V0x13,
                    parameters,
                )
                .hash_password_into(secret.as_bytes(), salt.as_bytes(), &mut writer_key)
                .unwrap();
            }
            let original = VaultData {
                entries: vec![Entry {
                    id: "fixture-id".into(),
                    title: "Historical OAuth".into(),
                    username: "person".into(),
                    password: "original credential".into(),
                    url: None,
                    icon_url: None,
                    totp_secret: None,
                    alias_provider_id: None,
                }],
                alias_configs: vec![],
                default_provider_id: None,
            };
            storage
                .write(&EncryptedVault {
                    version: "2".into(),
                    kdf: tag.into(),
                    salt: user_id.into(),
                    data: aead::encrypt(&writer_key, &serde_json::to_string(&original).unwrap())
                        .unwrap(),
                })
                .unwrap();
            let derived =
                Authenticator::derive_key(AuthCredential::OAuthToken(token.clone()), tag, user_id)
                    .unwrap();
            let mut workspace = Workspace::new();
            super::super::access::access(&storage, &mut workspace, &derived).unwrap();
            password(&storage, &mut workspace, "a new independent password").unwrap();
            workspace.lock();
            let header = storage.read().unwrap();
            let password_key = Authenticator::derive_key(
                AuthCredential::Password("a new independent password".into()),
                &header.kdf,
                &header.salt,
            )
            .unwrap();
            super::super::access::access(&storage, &mut workspace, &password_key).unwrap();
            assert_eq!(workspace.credentials[0].password, "original credential");
            assert_eq!(workspace.credentials[0].id, "fixture-id");
        }
    }

    #[test]
    fn historical_password_vault_rotates_without_losing_data_and_failed_rotation_preserves_access()
    {
        let directory = tempfile::tempdir().unwrap();
        let storage = VaultStorage {
            path: directory.path().join("vault.enc"),
        };
        // Parameters and 16-byte salt are the original pre-refactor writer's protocol.
        let salt = [7u8; 16];
        let parameters = argon2::Params::new(32768, 2, 2, None).unwrap();
        let algorithm = argon2::Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            parameters,
        );
        let mut old_key = [0u8; 32];
        algorithm
            .hash_password_into(b"old historical password", &salt, &mut old_key)
            .unwrap();
        let original = VaultData {
            entries: vec![Entry {
                id: "old-id".into(),
                title: "Historical".into(),
                username: "person".into(),
                password: "credential secret".into(),
                url: None,
                icon_url: None,
                totp_secret: Some("JBSWY3DPEHPK3PXP".into()),
                alias_provider_id: Some("simplelogin".into()),
            }],
            alias_configs: vec![AliasConfig {
                provider_id: "simplelogin".into(),
                api_token: "provider secret".into(),
                description: None,
            }],
            default_provider_id: Some("simplelogin".into()),
        };
        storage
            .write(&EncryptedVault {
                version: "2".into(),
                kdf: "argon2id".into(),
                salt: hex::encode(salt),
                data: aead::encrypt(&old_key, &serde_json::to_string(&original).unwrap()).unwrap(),
            })
            .unwrap();
        let derived = Authenticator::derive_key(
            AuthCredential::Password("old historical password".into()),
            "argon2id",
            &hex::encode(salt),
        )
        .unwrap();
        let mut workspace = Workspace::new();
        super::super::access::access(&storage, &mut workspace, &derived).unwrap();
        let previous = std::fs::read(&storage.path).unwrap();
        let failed_storage = VaultStorage {
            path: directory.path().join("missing").join("vault.enc"),
        };
        assert!(password(&failed_storage, &mut workspace, "a replacement password").is_err());
        assert_eq!(std::fs::read(&storage.path).unwrap(), previous);
        assert_eq!(workspace.credentials[0].password, "credential secret");
        password(&storage, &mut workspace, "a replacement password").unwrap();
        workspace.lock();
        assert!(super::super::access::access(&storage, &mut workspace, &old_key).is_err());
        let header = storage.read().unwrap();
        let new_key = Authenticator::derive_key(
            AuthCredential::Password("a replacement password".into()),
            &header.kdf,
            &header.salt,
        )
        .unwrap();
        super::super::access::access(&storage, &mut workspace, &new_key).unwrap();
        assert_eq!(workspace.credentials[0].id, "old-id");
        assert_eq!(
            workspace.credentials[0].totp_secret.as_deref(),
            Some("JBSWY3DPEHPK3PXP")
        );
        assert_eq!(workspace.alias_configs[0].api_token, "provider secret");
        assert_eq!(
            workspace.default_provider_id.as_deref(),
            Some("simplelogin")
        );
    }
}
