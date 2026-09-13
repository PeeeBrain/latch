use super::{storage::VaultStorage, workspace::Workspace, Entry, VaultData};
use crate::crypto::aead;
use zeroize::Zeroizing;

pub fn add(workspace: &mut Workspace, storage: &VaultStorage, entry: Entry) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    workspace.credentials.push(entry);
    persist(workspace, storage)
}

pub fn get_full(workspace: &mut Workspace, id: &str) -> Result<Entry, String> {
    workspace.check_session()?;
    workspace.refresh();
    workspace
        .credentials
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .ok_or_else(|| format!("Credential '{}' not found", id))
}

pub fn update(
    workspace: &mut Workspace,
    storage: &VaultStorage,
    mut entry: Entry,
) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    let idx = workspace
        .credentials
        .iter()
        .position(|e| e.id == entry.id)
        .ok_or_else(|| format!("Credential '{}' not found", entry.id))?;

    match entry.totp_secret.as_deref() {
        None => entry.totp_secret = workspace.credentials[idx].totp_secret.clone(),
        Some("") => entry.totp_secret = None,
        Some(_) => {}
    }
    workspace.credentials[idx] = entry;
    persist(workspace, storage)
}

pub fn delete(workspace: &mut Workspace, storage: &VaultStorage, id: &str) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    let len_before = workspace.credentials.len();
    workspace.credentials.retain(|e| e.id != id);
    if workspace.credentials.len() == len_before {
        return Err("Credential not found".to_string());
    }
    persist(workspace, storage)
}

pub fn get_field(workspace: &mut Workspace, id: &str, field: &str) -> Result<String, String> {
    workspace.check_session()?;
    workspace.refresh();
    let entry = workspace
        .credentials
        .iter()
        .find(|e| e.id == id)
        .ok_or("Credential not found".to_string())?;
    match field {
        "title" => Ok(entry.title.clone()),
        "username" => Ok(entry.username.clone()),
        "password" => Ok(entry.password.clone()),
        _ => Err("Field not found".to_string()),
    }
}

pub(crate) fn persist(workspace: &Workspace, storage: &VaultStorage) -> Result<(), String> {
    let key = workspace.session_key.as_ref().ok_or("Vault is locked")?;
    let vault_data = VaultData::from_workspace(workspace);
    let json = Zeroizing::new(
        serde_json::to_string(&vault_data).map_err(|e| format!("Failed to serialize: {}", e))?,
    );
    let encrypted = aead::encrypt(key, &json)?;

    let mut vault = storage.read()?;
    vault.data = encrypted;
    storage.write(&vault)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::aead::EncryptedData;
    use crate::vault::EncryptedVault;
    use std::time::{Duration, SystemTime};

    fn unlocked_workspace() -> Workspace {
        let mut workspace = Workspace::new();
        workspace.credentials.push(Entry {
            id: "entry-1".to_string(),
            title: "Example".to_string(),
            username: "user".to_string(),
            password: "secret".to_string(),
            url: None,
            icon_url: None,
            totp_secret: None,
        });
        workspace.start([7u8; 32]);
        workspace
    }

    fn test_storage() -> (VaultStorage, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = VaultStorage {
            path: dir.path().join("vault.enc"),
        };
        storage
            .write(&EncryptedVault {
                version: "1".to_string(),
                kdf: "argon2".to_string(),
                salt: "00".to_string(),
                data: EncryptedData {
                    nonce: "00".to_string(),
                    ciphertext: "00".to_string(),
                },
            })
            .unwrap();
        (storage, dir)
    }

    fn replacement(totp_secret: Option<&str>) -> Entry {
        Entry {
            id: "entry-1".to_string(),
            title: "Renamed".to_string(),
            username: "user".to_string(),
            password: "new-secret".to_string(),
            url: None,
            icon_url: None,
            totp_secret: totp_secret.map(str::to_string),
        }
    }

    #[test]
    fn update_preserves_a_stored_totp_secret_when_the_replacement_omits_it() {
        let (storage, _dir) = test_storage();
        let mut workspace = unlocked_workspace();
        workspace.credentials[0].totp_secret = Some("JBSWY3DPEHPK3PXP".to_string());

        update(&mut workspace, &storage, replacement(None)).unwrap();

        assert_eq!(workspace.credentials[0].title, "Renamed");
        assert_eq!(
            workspace.credentials[0].totp_secret.as_deref(),
            Some("JBSWY3DPEHPK3PXP")
        );
    }

    #[test]
    fn update_replaces_the_stored_totp_secret_when_a_new_one_is_provided() {
        let (storage, _dir) = test_storage();
        let mut workspace = unlocked_workspace();
        workspace.credentials[0].totp_secret = Some("OLD".to_string());

        update(&mut workspace, &storage, replacement(Some("NEW"))).unwrap();

        assert_eq!(workspace.credentials[0].totp_secret.as_deref(), Some("NEW"));
    }

    #[test]
    fn update_clears_the_stored_totp_secret_when_an_empty_value_is_provided() {
        let (storage, _dir) = test_storage();
        let mut workspace = unlocked_workspace();
        workspace.credentials[0].totp_secret = Some("JBSWY3DPEHPK3PXP".to_string());

        update(&mut workspace, &storage, replacement(Some(""))).unwrap();

        assert_eq!(workspace.credentials[0].totp_secret, None);
    }

    #[test]
    fn get_full_rejects_expired_session() {
        let mut workspace = unlocked_workspace();
        workspace.session_start =
            Some(SystemTime::now() - Duration::from_secs(super::super::SESSION_TIMEOUT_SECS + 1));

        let result = get_full(&mut workspace, "entry-1");

        assert_eq!(result.unwrap_err(), "Session expired");
        assert!(workspace.session_key.is_none());
        assert!(workspace.credentials.is_empty());
    }
}
