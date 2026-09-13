pub mod access;
pub mod coordinator;
pub mod entries;
pub mod provision;
pub mod rotate;
pub mod search;
pub mod storage;
pub mod totp;
pub mod workspace;

use serde::{Deserialize, Serialize};

pub const SESSION_TIMEOUT_SECS: u64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub has_totp: bool,
}

impl From<Entry> for EntryPreview {
    fn from(entry: Entry) -> Self {
        EntryPreview {
            id: entry.id,
            title: entry.title,
            username: entry.username,
            icon_url: entry.icon_url,
            has_totp: entry.totp_secret.is_some(),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultData {
    pub entries: Vec<Entry>,
}
