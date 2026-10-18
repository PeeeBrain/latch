use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub enum AuthCredential {
    Password(String),
    #[cfg(feature = "legacy-oauth")]
    OAuthToken(String),
    RawKeyHex(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    CredentialMismatch,
    InvalidSalt,
    #[cfg(feature = "legacy-oauth")]
    InvalidToken(String),
    InvalidKey,
    LegacyOAuth,
}

impl Display for AuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CredentialMismatch => {
                formatter.write_str("credential does not match vault auth method")
            }
            Self::InvalidSalt => formatter.write_str("invalid password salt"),
            #[cfg(feature = "legacy-oauth")]
            Self::InvalidToken(error) => write!(formatter, "invalid OAuth token: {error}"),
            Self::InvalidKey => formatter.write_str("invalid biometric key"),
            Self::LegacyOAuth => formatter
                .write_str("Legacy Google vault requires migration in the previous Latch app"),
        }
    }
}

impl Error for AuthError {}

pub struct Authenticator;

impl Authenticator {
    pub fn derive_key(
        credential: AuthCredential,
        kdf_tag: &str,
        salt_hex: &str,
    ) -> Result<[u8; 32], AuthError> {
        #[cfg(not(feature = "legacy-oauth"))]
        if matches!(kdf_tag, "oauth-argon2id" | "oauth-pbkdf2") {
            return Err(AuthError::LegacyOAuth);
        }
        match (credential, kdf_tag) {
            (AuthCredential::Password(password), "password-pbkdf2") => {
                let password = zeroize::Zeroizing::new(password);
                let salt_bytes = hex::decode(salt_hex).map_err(|_| AuthError::InvalidSalt)?;
                let salt: [u8; 32] = salt_bytes.try_into().map_err(|_| AuthError::InvalidSalt)?;
                Ok(crate::auth::password::derive_key(&password, &salt))
            }
            (AuthCredential::Password(password), "argon2id") => {
                let password = zeroize::Zeroizing::new(password);
                let salt = hex::decode(salt_hex).map_err(|_| AuthError::InvalidSalt)?;
                if salt.len() != 16 {
                    return Err(AuthError::InvalidSalt);
                }
                let parameters =
                    argon2::Params::new(32768, 2, 2, None).map_err(|_| AuthError::InvalidSalt)?;
                let algorithm = argon2::Argon2::new(
                    argon2::Algorithm::Argon2id,
                    argon2::Version::V0x13,
                    parameters,
                );
                let mut key = [0; 32];
                algorithm
                    .hash_password_into(password.as_bytes(), &salt, &mut key)
                    .map_err(|_| AuthError::InvalidSalt)?;
                Ok(key)
            }
            #[cfg(feature = "legacy-oauth")]
            (AuthCredential::OAuthToken(token), tag @ ("oauth-argon2id" | "oauth-pbkdf2")) => {
                let user_id =
                    crate::auth::oauth::extract_user_id(&token).map_err(AuthError::InvalidToken)?;
                if tag == "oauth-pbkdf2" {
                    crate::auth::oauth::derive_historical_key(&user_id)
                        .map_err(AuthError::InvalidToken)
                } else {
                    crate::auth::oauth::derive_key(&user_id).map_err(AuthError::InvalidToken)
                }
            }
            (AuthCredential::RawKeyHex(key_hex), "biometric-keychain") => {
                let key_hex = zeroize::Zeroizing::new(key_hex);
                let key = zeroize::Zeroizing::new(
                    hex::decode(key_hex.as_str()).map_err(|_| AuthError::InvalidKey)?,
                );
                key.as_slice().try_into().map_err(|_| AuthError::InvalidKey)
            }
            _ => Err(AuthError::CredentialMismatch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthCredential, AuthError, Authenticator};
    #[cfg(feature = "legacy-oauth")]
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    #[cfg(feature = "legacy-oauth")]
    use serde_json::json;

    #[cfg(feature = "legacy-oauth")]
    fn id_token(user_id: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(json!({ "alg": "RS256" }).to_string());
        let claims = URL_SAFE_NO_PAD.encode(
            json!({
                "sub": user_id,
                "iss": "accounts.google.com",
                "exp": 4_102_444_800_u64,
            })
            .to_string(),
        );
        format!("{header}.{claims}.signature")
    }

    #[test]
    fn password_credential_derives_the_existing_password_key() {
        let salt = [7_u8; 32];

        let key = Authenticator::derive_key(
            AuthCredential::Password("correct horse".to_string()),
            "password-pbkdf2",
            &hex::encode(salt),
        )
        .unwrap();

        assert_eq!(
            key,
            crate::auth::password::derive_key("correct horse", &salt)
        );
    }

    #[cfg(feature = "legacy-oauth")]
    #[test]
    fn oauth_credential_derives_the_existing_oauth_key() {
        let token = id_token("google-user-42");

        let key = Authenticator::derive_key(
            AuthCredential::OAuthToken(token),
            "oauth-argon2id",
            "google-user-42",
        )
        .unwrap();

        assert_eq!(
            key,
            crate::auth::oauth::derive_key("google-user-42").unwrap()
        );
    }

    #[test]
    fn biometric_credential_decodes_the_raw_key() {
        let expected = [11_u8; 32];

        let key = Authenticator::derive_key(
            AuthCredential::RawKeyHex(hex::encode(expected)),
            "biometric-keychain",
            "",
        )
        .unwrap();

        assert_eq!(key, expected);
    }

    #[test]
    fn credential_must_match_the_vault_auth_method() {
        let error = Authenticator::derive_key(
            AuthCredential::Password("secret".to_string()),
            "biometric-keychain",
            "",
        )
        .unwrap_err();

        assert_eq!(error, AuthError::CredentialMismatch);
    }

    #[test]
    fn password_credential_rejects_invalid_salt_hex() {
        let error = Authenticator::derive_key(
            AuthCredential::Password("secret".to_string()),
            "password-pbkdf2",
            "not-hex",
        )
        .unwrap_err();

        assert_eq!(error, AuthError::InvalidSalt);
    }

    #[cfg(feature = "legacy-oauth")]
    #[test]
    fn oauth_credential_rejects_a_malformed_token() {
        let error = Authenticator::derive_key(
            AuthCredential::OAuthToken("not-a-token".to_string()),
            "oauth-argon2id",
            "",
        )
        .unwrap_err();

        assert!(matches!(error, AuthError::InvalidToken(_)));
    }

    #[cfg(not(feature = "legacy-oauth"))]
    #[test]
    fn legacy_google_vaults_require_migration() {
        for tag in ["oauth-argon2id", "oauth-pbkdf2"] {
            let error =
                Authenticator::derive_key(AuthCredential::Password("secret".to_string()), tag, "")
                    .unwrap_err();

            assert_eq!(
                error.to_string(),
                "Legacy Google vault requires migration in the previous Latch app"
            );
        }
    }
}
