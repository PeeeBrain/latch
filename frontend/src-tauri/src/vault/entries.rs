use super::{storage::VaultStorage, workspace::Workspace, Entry, VaultData};
use crate::crypto::aead;

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
    entry: Entry,
) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    let idx = workspace
        .credentials
        .iter()
        .position(|e| e.id == entry.id)
        .ok_or_else(|| format!("Credential '{}' not found", entry.id))?;
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

fn persist(workspace: &Workspace, storage: &VaultStorage) -> Result<(), String> {
    let key = workspace.session_key.as_ref().ok_or("Vault is locked")?;
    let vault_data = VaultData::from_workspace(workspace);
    let json =
        serde_json::to_string(&vault_data).map_err(|e| format!("Failed to serialize: {}", e))?;
    let encrypted = aead::encrypt(key, &json)?;

    let mut vault = storage.read()?;
    vault.data = encrypted;
    storage.write(&vault)
}

#[cfg(test)]
mod tests {
    use super::*;
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
