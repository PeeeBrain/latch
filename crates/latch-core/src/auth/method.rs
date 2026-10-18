use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    OAuth,
    Biometric,
}

impl AuthMethod {
    pub fn vault_tag(&self) -> &'static str {
        match self {
            AuthMethod::Password => "password-pbkdf2",
            AuthMethod::OAuth => "oauth-argon2id",
            AuthMethod::Biometric => "biometric-keychain",
        }
    }

    pub fn from_vault_tag(tag: &str) -> Option<Self> {
        match tag {
            "password-pbkdf2" | "argon2id" => Some(AuthMethod::Password),
            "oauth-argon2id" | "oauth-pbkdf2" => Some(AuthMethod::OAuth),
            "biometric-keychain" => Some(AuthMethod::Biometric),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn all_tags() -> &'static [&'static str] {
        &[
            "password-pbkdf2",
            "argon2id",
            "oauth-argon2id",
            "oauth-pbkdf2",
            "biometric-keychain",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_all_methods() {
        for method in &[
            AuthMethod::Password,
            AuthMethod::OAuth,
            AuthMethod::Biometric,
        ] {
            let tag = method.vault_tag();
            let parsed = AuthMethod::from_vault_tag(tag).unwrap();
            assert_eq!(*method, parsed);
        }
    }

    #[test]
    fn test_oauth_legacy_pbkdf2_tag() {
        let method = AuthMethod::from_vault_tag("oauth-pbkdf2").unwrap();
        assert_eq!(method, AuthMethod::OAuth);
    }

    #[test]
    fn test_unknown_tag() {
        assert!(AuthMethod::from_vault_tag("unknown").is_none());
    }
}
