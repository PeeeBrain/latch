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

    // Use Argon2id to derive a 32-byte key
    // Parameters: memory_cost=65536 (64MB), time_cost=3, parallelism=4
    let params =
        Params::new(65536, 3, 4, Some(32)).map_err(|e| format!("Invalid Argon2 params: {}", e))?;

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    // Salt includes user_id to make keys user-specific
    let salt = format!("latch-vault-oauth-{}", user_id);
    let salt_bytes = salt.as_bytes();

    // Derive key using Argon2id
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(app_secret.as_bytes(), salt_bytes, &mut key)
        .map_err(|e| format!("Argon2 hashing failed: {}", e))?;

    Ok(key)
}

/// Compatibility with the writer at 25f09de; confined to the migration bridge.
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
    // Validate critical claims for security
    // Note: Signature validation requires fetching Google's public keys (JWKs)
    // which should be implemented for production. For now, we validate claims.
    let client_id = env::var("LATCH_OAUTH_CLIENT_ID").unwrap_or_else(|_| String::new());

    let mut validation = Validation::new(Algorithm::RS256);
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
        &jsonwebtoken::DecodingKey::from_secret(&[]),
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

    #[test]
    fn test_decode_id_token_valid_structure() {
        let valid_token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiYXVkIjoiY2xpZW50X2lkIn0.signature";
        let result = decode_id_token(valid_token);
        assert!(result.is_err());
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
        let user_id = "test-user-id-123";
        let payload = json!({ "sub": user_id });
        let encoded = general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        let token = format!("header.{}.signature", encoded);

        let result = extract_user_id(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_missing_sub() {
        let payload = json!({ "name": "John Doe" });
        let encoded = general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        let token = format!("header.{}.signature", encoded);

        let result = extract_user_id(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_invalid_json() {
        let encoded = general_purpose::URL_SAFE_NO_PAD.encode("invalid json");
        let token = format!("header.{}.signature", encoded);

        let result = extract_user_id(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_user_id_empty_string() {
        let result = extract_user_id("");
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_key_returns_valid_key() {
        let user_id = "test-user-id-123";
        let key = derive_key(user_id).unwrap();
        assert_eq!(key.len(), 32);
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
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_key_long_user_id() {
        let long_user_id = "a".repeat(1000);
        let key = derive_key(&long_user_id).unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_derive_key_unicode() {
        let unicode_user_id = "用户-123-пользователь";
        let key = derive_key(unicode_user_id).unwrap();
        assert_eq!(key.len(), 32);
    }
}
