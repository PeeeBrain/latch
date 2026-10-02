use super::{storage::VaultStorage, workspace::Workspace, Entry, VaultData};
use crate::crypto::aead;
use zeroize::Zeroizing;

pub fn add(
    workspace: &mut Workspace,
    storage: &VaultStorage,
    mut entry: Entry,
) -> Result<(), String> {
    workspace.check_session()?;
    validate(&mut entry)?;
    if workspace
        .credentials
        .iter()
        .any(|stored| stored.id == entry.id)
    {
        return Err("Credential already exists".into());
    }
    workspace.refresh();
    workspace.credentials.push(entry);
    if let Err(error) = persist(workspace, storage) {
        workspace.credentials.pop();
        return Err(error);
    }
    Ok(())
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
    validate(&mut entry)?;
    let previous = std::mem::replace(&mut workspace.credentials[idx], entry);
    if let Err(error) = persist(workspace, storage) {
        workspace.credentials[idx] = previous;
        return Err(error);
    }
    Ok(())
}

pub fn delete(workspace: &mut Workspace, storage: &VaultStorage, id: &str) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    let index = workspace
        .credentials
        .iter()
        .position(|entry| entry.id == id)
        .ok_or("Credential not found")?;
    let previous = workspace.credentials.remove(index);
    if let Err(error) = persist(workspace, storage) {
        workspace.credentials.insert(index, previous);
        return Err(error);
    }
    Ok(())
}

fn validate(entry: &mut Entry) -> Result<(), String> {
    for (name, value, limit) in [
        ("Title", entry.title.as_str(), 256),
        ("Username", entry.username.as_str(), 256),
        ("Password", entry.password.as_str(), 1024),
    ] {
        if value.trim().is_empty() || value.len() > limit {
            return Err(format!(
                "{name} is required and must be at most {limit} bytes"
            ));
        }
    }
    if entry.id.is_empty() || entry.id.len() > 256 {
        return Err("Invalid credential ID".into());
    }
    if let Some(value) = entry
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        let parsed = url::Url::parse(value).map_err(|_| "Invalid URL")?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("URL must use HTTP or HTTPS".into());
        }
    }
    if let Some(secret) = entry
        .totp_secret
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        entry.totp_secret = Some(super::totp::extract_secret(secret)?);
    }
    Ok(())
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
            alias_provider_id: None,
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
            alias_provider_id: None,
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

        update(
            &mut workspace,
            &storage,
            replacement(Some("GEZDGNBVGY3TQOJQ")),
        )
        .unwrap();

        assert_eq!(
            workspace.credentials[0].totp_secret.as_deref(),
            Some("GEZDGNBVGY3TQOJQ")
        );
    }

    #[test]
    fn update_replaces_the_stored_alias_provider_id() {
        let (storage, _dir) = test_storage();
        let mut workspace = unlocked_workspace();
        workspace.credentials[0].alias_provider_id = Some("simplelogin".to_string());

        update(&mut workspace, &storage, replacement(None)).unwrap();

        assert_eq!(workspace.credentials[0].alias_provider_id, None);
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

    #[test]
    fn rejected_or_failed_edits_preserve_memory_and_disk() {
        let (storage, directory) = test_storage();
        let original = std::fs::read(&storage.path).unwrap();
        let mut workspace = unlocked_workspace();
        let mut invalid = replacement(None);
        invalid.title.clear();
        assert!(update(&mut workspace, &storage, invalid).is_err());
        assert_eq!(workspace.credentials[0].title, "Example");
        assert_eq!(std::fs::read(&storage.path).unwrap(), original);
        let inaccessible = VaultStorage {
            path: directory.path().to_path_buf(),
        };
        assert!(update(&mut workspace, &inaccessible, replacement(None)).is_err());
        assert_eq!(workspace.credentials[0].title, "Example");
        assert!(delete(&mut workspace, &inaccessible, "entry-1").is_err());
        assert_eq!(workspace.credentials.len(), 1);
        let mut extra = replacement(None);
        extra.id = "entry-2".into();
        assert!(add(&mut workspace, &inaccessible, extra).is_err());
        assert_eq!(workspace.credentials.len(), 1);
    }
}
