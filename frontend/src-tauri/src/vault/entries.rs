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
    totp_secret: Option<String>,
) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    let idx = workspace
        .credentials
        .iter()
        .position(|e| e.id == entry.id)
        .ok_or_else(|| format!("Credential '{}' not found", entry.id))?;

    let mut entry = entry;
    entry.totp_secret = merge_totp(workspace.credentials[idx].totp_secret.clone(), totp_secret)?;
    workspace.credentials[idx] = entry;
    persist(workspace, storage)
}

/// TOTP merge semantics: `None` keeps the stored secret, an empty value clears
/// it, and any other value replaces it (validated/normalized).
fn merge_totp(current: Option<String>, update: Option<String>) -> Result<Option<String>, String> {
    match update {
        None => Ok(current),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => Ok(Some(super::totp::extract_secret(&value)?)),
    }
}

pub fn totp_token(
    workspace: &mut Workspace,
    id: &str,
    unix_time: u64,
) -> Result<(String, u64), String> {
    workspace.check_session()?;
    workspace.refresh();
    let entry = workspace
        .credentials
        .iter()
        .find(|e| e.id == id)
        .ok_or("Credential not found".to_string())?;
    let secret = entry
        .totp_secret
        .as_deref()
        .ok_or("TOTP is not configured for this credential".to_string())?;

    let token = super::totp::generate_token(secret, unix_time)?;
    Ok((token, super::totp::remaining_seconds(unix_time)))
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
    let vault_data = VaultData {
        entries: workspace.credentials.clone(),
    };
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

    #[test]
    fn merge_totp_keeps_stored_secret_when_update_is_none() {
        let current = Some("GEZDGNBV".to_string());
        assert_eq!(merge_totp(current.clone(), None).unwrap(), current);
        assert_eq!(merge_totp(None, None).unwrap(), None);
    }

    #[test]
    fn merge_totp_clears_blank_and_normalizes_real_values() {
        assert_eq!(
            merge_totp(Some("OLD".into()), Some("   ".into())).unwrap(),
            None
        );
        assert_eq!(
            merge_totp(None, Some(" gezd-gnbv ".into())).unwrap(),
            Some("GEZDGNBV".to_string())
        );
    }

    #[test]
    fn merge_totp_rejects_invalid_secret() {
        assert!(merge_totp(None, Some("GEZD1NBV".into())).is_err());
    }

    #[test]
    fn totp_token_errors_when_not_configured() {
        let mut workspace = unlocked_workspace();

        let error = totp_token(&mut workspace, "entry-1", 59).unwrap_err();

        assert_eq!(error, "TOTP is not configured for this credential");
    }

    #[test]
    fn totp_token_generates_token_and_remaining_seconds() {
        let mut workspace = unlocked_workspace();
        workspace.credentials[0].totp_secret = Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string());

        let (token, remaining) = totp_token(&mut workspace, "entry-1", 59).unwrap();

        assert_eq!(token, "287082");
        assert_eq!(remaining, 1);
    }

    #[test]
    fn totp_token_rejects_expired_session() {
        let mut workspace = unlocked_workspace();
        workspace.session_start =
            Some(SystemTime::now() - Duration::from_secs(super::super::SESSION_TIMEOUT_SECS + 1));

        let result = totp_token(&mut workspace, "entry-1", 59);

        assert_eq!(result.unwrap_err(), "Session expired");
    }
}
