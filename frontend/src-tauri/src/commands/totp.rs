use crate::commands::VaultState;
use crate::vault::workspace::Workspace;
use serde::Serialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Serialize)]
struct TotpToken {
    token: String,
    remaining_seconds: u64,
}

fn token_for_entry(
    workspace: &mut Workspace,
    entry_id: &str,
    timestamp: u64,
) -> Result<TotpToken, String> {
    workspace.check_session()?;
    workspace.refresh();
    let secret = workspace
        .credentials
        .iter()
        .find(|entry| entry.id == entry_id)
        .ok_or_else(|| format!("Credential '{entry_id}' not found"))?
        .totp_secret
        .as_deref()
        .ok_or("Credential has no TOTP secret")?;

    Ok(TotpToken {
        token: crate::vault::totp::generate_token(secret, timestamp)?,
        remaining_seconds: crate::vault::totp::remaining_seconds(timestamp),
    })
}

#[tauri::command]
pub async fn get_totp_token(
    entry_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "System clock is before the Unix epoch")?
        .as_secs();
    let response = state.lock(|_, workspace| token_for_entry(workspace, &entry_id, timestamp))?;

    Ok(json!({
        "status": "success",
        "token": response.token,
        "remaining_seconds": response.remaining_seconds,
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use crate::vault::{workspace::Workspace, Entry};

    #[test]
    fn returns_a_token_without_exposing_the_stored_secret() {
        let mut workspace = Workspace::new();
        workspace.credentials.push(Entry {
            id: "credential-1".to_string(),
            title: "Example".to_string(),
            username: "user".to_string(),
            password: "password".to_string(),
            url: None,
            icon_url: None,
            totp_secret: Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".to_string()),
            alias_provider_id: None,
        });
        workspace.start([7; 32]);

        let response = super::token_for_entry(&mut workspace, "credential-1", 59).unwrap();

        assert_eq!(response.token, "287082");
        assert_eq!(response.remaining_seconds, 1);
        assert!(!serde_json::to_string(&response).unwrap().contains("GEZD"));
    }
}
