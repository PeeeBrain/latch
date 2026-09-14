use crate::commands::VaultState;
use crate::vault_health::breach_checker::PwnedPasswordsApi;
use serde_json::json;
use tauri::State;

fn session_checked_entries(
    workspace: &mut crate::vault::workspace::Workspace,
) -> Result<Vec<crate::vault::Entry>, String> {
    workspace.check_session()?;
    workspace.refresh();
    Ok(workspace.credentials.clone())
}

#[tauri::command]
pub async fn check_vault_health(state: State<'_, VaultState>) -> Result<String, String> {
    let entries = state.lock(|_, workspace| session_checked_entries(workspace))?;

    let checker = PwnedPasswordsApi;
    let report = crate::vault_health::audit::check_vault_health(&entries, &checker).await;

    Ok(json!({
        "status": "success",
        "report": report
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use crate::vault::{workspace::Workspace, Entry};
    use std::time::{Duration, SystemTime};

    #[test]
    fn health_entries_reject_expired_session() {
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
        workspace.start([5u8; 32]);
        workspace.session_start =
            Some(SystemTime::now() - Duration::from_secs(crate::vault::SESSION_TIMEOUT_SECS + 1));

        let result = super::session_checked_entries(&mut workspace);

        assert_eq!(result.unwrap_err(), "Session expired");
        assert!(!workspace.is_unlocked());
    }
}
