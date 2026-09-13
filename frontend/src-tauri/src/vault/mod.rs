pub mod access;
pub mod alias;
pub mod coordinator;
pub mod entries;
pub mod provision;
pub mod rotate;
pub mod search;
pub mod storage;
pub mod totp;
pub mod workspace;

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const SESSION_TIMEOUT_SECS: u64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub username: String,
    pub password: String,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub totp_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPreview {
    pub id: String,
    pub title: String,
    pub username: String,
    pub icon_url: Option<String>,
}

impl From<&Entry> for EntryPreview {
    fn from(entry: &Entry) -> Self {
        EntryPreview {
            id: entry.id.clone(),
            title: entry.title.clone(),
            username: entry.username.clone(),
            icon_url: entry.icon_url.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedVault {
    pub version: String,
    pub kdf: String,
    pub salt: String,
    pub data: crate::crypto::aead::EncryptedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct AliasConfig {
    pub provider_id: String,
    pub api_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultData {
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub alias_configs: Vec<AliasConfig>,
    #[serde(default)]
    pub default_provider_id: Option<String>,
}

impl VaultData {
    pub fn from_workspace(workspace: &workspace::Workspace) -> Self {
        Self {
            entries: workspace.credentials.clone(),
            alias_configs: workspace.alias_configs.clone(),
            default_provider_id: workspace.default_provider_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AliasConfig, Entry, VaultData};

    #[test]
    fn alias_configs_round_trip_through_the_vault_data_schema() {
        let vault_data = VaultData {
            entries: Vec::new(),
            alias_configs: vec![AliasConfig {
                provider_id: "simplelogin".to_string(),
                api_token: "sl-token".to_string(),
            }],
            default_provider_id: Some("simplelogin".to_string()),
        };

        let json = serde_json::to_string(&vault_data).unwrap();
        let parsed: VaultData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.alias_configs[0].provider_id, "simplelogin");
        assert_eq!(parsed.alias_configs[0].api_token, "sl-token");
        assert_eq!(parsed.default_provider_id.as_deref(), Some("simplelogin"));
    }

    #[test]
    fn vaults_without_alias_configs_still_parse() {
        let legacy = r#"{"entries":[]}"#;
        let parsed: VaultData = serde_json::from_str(legacy).unwrap();

        assert!(parsed.alias_configs.is_empty());
        assert!(parsed.default_provider_id.is_none());
    }

    #[test]
    fn credential_secrets_are_zeroized_in_place() {
        use zeroize::Zeroize;

        let mut entry = Entry {
            id: "1".to_string(),
            title: "Example".to_string(),
            username: "user".to_string(),
            password: "secret".to_string(),
            url: None,
            icon_url: None,
            totp_secret: Some("JBSWY3DPEHPK3PXP".to_string()),
        };

        entry.zeroize();

        assert!(entry.password.is_empty());
        assert!(entry.totp_secret.is_none());
    }

    #[test]
    fn credential_totp_secret_round_trips_and_remains_optional() {
        let json = r#"{"id":"1","title":"Example","username":"user","password":"secret","url":null,"icon_url":null,"totp_secret":"JBSWY3DPEHPK3PXP"}"#;
        let credential: Entry = serde_json::from_str(json).unwrap();

        assert_eq!(credential.totp_secret.as_deref(), Some("JBSWY3DPEHPK3PXP"));
        assert_eq!(
            serde_json::to_value(&credential).unwrap()["totp_secret"],
            "JBSWY3DPEHPK3PXP"
        );

        let legacy = r#"{"id":"2","title":"Legacy","username":"user","password":"secret","url":null,"icon_url":null}"#;
        assert!(serde_json::from_str::<Entry>(legacy)
            .unwrap()
            .totp_secret
            .is_none());
    }
}
