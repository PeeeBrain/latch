use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub enum AuthCredential {
    Password(String),
    OAuthToken(String),
    RawKeyHex(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthError {
    CredentialMismatch,
    InvalidSalt,
    InvalidToken(String),
    InvalidKey,
}

impl Display for AuthError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CredentialMismatch => {
                formatter.write_str("credential does not match vault auth method")
            }
            Self::InvalidSalt => formatter.write_str("invalid password salt"),
            Self::InvalidToken(error) => write!(formatter, "invalid OAuth token: {error}"),
            Self::InvalidKey => formatter.write_str("invalid biometric key"),
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
        match (credential, kdf_tag) {
            (AuthCredential::Password(password), "password-pbkdf2") => {
                let salt_bytes = hex::decode(salt_hex).map_err(|_| AuthError::InvalidSalt)?;
                let salt: [u8; 32] = salt_bytes.try_into().map_err(|_| AuthError::InvalidSalt)?;
                Ok(crate::auth::password::derive_key(&password, &salt))
            }
            (AuthCredential::OAuthToken(token), "oauth-argon2id" | "oauth-pbkdf2") => {
                let user_id =
                    crate::auth::oauth::extract_user_id(&token).map_err(AuthError::InvalidToken)?;
                crate::auth::oauth::derive_key(&user_id).map_err(AuthError::InvalidToken)
            }
            (AuthCredential::RawKeyHex(key_hex), "biometric-keychain") => {
                let key = hex::decode(key_hex).map_err(|_| AuthError::InvalidKey)?;
                key.try_into().map_err(|_| AuthError::InvalidKey)
            }
            _ => Err(AuthError::CredentialMismatch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthCredential, AuthError, Authenticator};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use serde_json::json;

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
            "oauth-argon2id",
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
}
