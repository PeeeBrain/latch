use argon2::{Argon2, Params};
use jsonwebtoken::{decode, Algorithm, Validation};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct GoogleIdToken {
    pub sub: String,
}

fn get_app_secret() -> String {
    let secret = env::var("LATCH_OAUTH_SECRET")
        .unwrap_or_else(|_| "test-secret-for-development-only-32b".to_string());

    if secret.len() < 32 {
        panic!(
            "LATCH_OAUTH_SECRET must be at least 32 bytes for security. Current length: {}",
            secret.len()
        )
    }

    secret
}

pub fn derive_key(user_id: &str) -> Result<[u8; 32], String> {
    let app_secret = get_app_secret();

    let params =
        Params::new(65536, 3, 4, Some(32)).map_err(|e| format!("Invalid Argon2 params: {}", e))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let salt = format!("latch-vault-oauth-{}", user_id);
    let salt_bytes = salt.as_bytes();

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(app_secret.as_bytes(), salt_bytes, &mut key)
        .map_err(|e| format!("Argon2 hashing failed: {}", e))?;

    Ok(key)
}

/// Compatibility with the writer at 25f09de; used only by legacy vault tests.
pub fn derive_historical_key(user_id: &str) -> Result<[u8; 32], String> {
    let secret = zeroize::Zeroizing::new(
        env::var("LATCH_OAUTH_SECRET").unwrap_or_else(|_| "latch-dev-secret-32bytes-long!!".into()),
    );
    if secret.len() < 16 {
        return Err("Original OAuth secret is too short".into());
    }
    let mut key = [0; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(
        secret.as_bytes(),
        format!("latch-vault-oauth-{user_id}").as_bytes(),
        100_000,
        &mut key,
    );
    Ok(key)
}

pub fn decode_id_token(id_token: &str) -> Result<GoogleIdToken, String> {
    // Historical compatibility only; the native app never enables this module.
    let client_id = env::var("LATCH_OAUTH_CLIENT_ID").unwrap_or_else(|_| String::new());

    let mut validation = Validation::new(Algorithm::RS256);
    // insecure_decode skips claim validation too, so retain the historical checks.
    #[expect(
        deprecated,
        reason = "Historical Vault tests still need claim validation"
    )]
    validation.insecure_disable_signature_validation();
    validation.validate_aud = true;
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);

    if !client_id.is_empty() {
        validation.set_audience(&[&client_id]);
    }

    let token_data = decode::<GoogleIdToken>(
        id_token,
        &jsonwebtoken::DecodingKey::from_rsa_der(&[]),
        &validation,
    )
    .map_err(|e| format!("Failed to decode token: {}", e))?;

    Ok(token_data.claims)
}

pub fn extract_user_id(id_token: &str) -> Result<String, String> {
    let claims = decode_id_token(id_token)?;
    Ok(claims.sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn token(payload: &str) -> String {
        let header = general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = general_purpose::URL_SAFE_NO_PAD.encode(payload);
        let signature = general_purpose::URL_SAFE_NO_PAD.encode("signature");
        format!("{header}.{payload}.{signature}")
    }

    fn claims(sub: Option<&str>) -> serde_json::Value {
        let mut claims = json!({
            "exp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
            "iss": "https://accounts.google.com",
        });
        if let Ok(client_id) = env::var("LATCH_OAUTH_CLIENT_ID") {
            claims["aud"] = json!(client_id);
        }
        if let Some(sub) = sub {
            claims["sub"] = json!(sub);
        }
        claims
    }

    #[test]
    fn test_decode_id_token_valid_claims() {
        let encoded = token(&claims(Some("1234567890")).to_string());

        let result = decode_id_token(&encoded).unwrap();
        assert_eq!(result.sub, "1234567890");
        assert_eq!(extract_user_id(&encoded).unwrap(), "1234567890");
    }

    #[test]
    fn test_decode_id_token_rejects_incomplete_claims() {
        let incomplete = json!({ "sub": "1234567890" });
        let incomplete = if let Ok(client_id) = env::var("LATCH_OAUTH_CLIENT_ID") {
            json!({ "sub": "1234567890", "aud": client_id })
        } else {
            incomplete
        };

        assert!(decode_id_token(&token(&incomplete.to_string())).is_err());
    }

    #[test]
    fn test_decode_id_token_invalid_format() {
        let invalid_token = "invalid.token.format";
        let result = decode_id_token(invalid_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_id_token_empty() {
        let result = decode_id_token("");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_id_token_no_payload() {
        let result = decode_id_token("header.");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_valid() {
        let result = extract_user_id(&token(&claims(Some("test-user-id-123")).to_string()));
        assert_eq!(result.unwrap(), "test-user-id-123");
    }

    #[test]
    fn test_extract_user_id_missing_sub() {
        let encoded = token(&claims(None).to_string());

        assert!(extract_user_id(&encoded).is_err());
    }

    #[test]
    fn test_extract_user_id_invalid_json() {
        let token = token("invalid json");

        let result = extract_user_id(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_empty_string() {
        let result = extract_user_id("");
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_is_deterministic() {
        let user_id = "test-user-id-123";
        let key = derive_key(user_id).unwrap();
        assert_eq!(key, derive_key(user_id).unwrap());
    }

    #[test]
    fn test_derive_key_different_users() {
        let user_id_1 = "user-1";
        let user_id_2 = "user-2";

        let key1 = derive_key(user_id_1).unwrap();
        let key2 = derive_key(user_id_2).unwrap();

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_empty_user_id() {
        let key = derive_key("").unwrap();
        assert_ne!(key, derive_key(" ").unwrap());
    }

    #[test]
    fn test_derive_key_long_user_id() {
        let long_user_id = "a".repeat(1000);
        let key = derive_key(&long_user_id).unwrap();
        let changed_user_id = format!("{}b", "a".repeat(999));
        assert_ne!(key, derive_key(&changed_user_id).unwrap());
    }

    #[test]
    fn test_derive_key_unicode() {
        let unicode_user_id = "用户-123-пользователь";
        let key = derive_key(unicode_user_id).unwrap();
        assert_ne!(key, derive_key("用户-123-пользователе").unwrap());
    }
}
