use crate::commands::VaultState;
use serde_json::json;
use tauri::State;

fn session_remaining_seconds(workspace: &mut crate::vault::workspace::Workspace) -> u64 {
    workspace
        .session_id()
        .and_then(|generation| workspace.expire_session(generation, std::time::SystemTime::now()))
        .unwrap_or_default()
        .as_secs()
}

#[tauri::command]
pub async fn lock_vault(state: State<'_, VaultState>) -> Result<String, String> {
    state.lock(|_, workspace| {
        workspace.lock();
        Ok(())
    })?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn get_auth_preferences(state: State<'_, VaultState>) -> Result<String, String> {
    state.lock(|storage, workspace| {
        let auth_method = if storage.exists() {
            storage
                .read()
                .map(|v| v.kdf)
                .unwrap_or_else(|_| "none".to_string())
        } else {
            "none".to_string()
        };
        let session_remaining = session_remaining_seconds(workspace);
        let is_unlocked = workspace.is_unlocked();

        Ok(json!({
            "status": "success",
            "auth_method": auth_method,
            "session_valid": is_unlocked,
            "session_remaining_seconds": session_remaining
        })
        .to_string())
    })
}

#[cfg(test)]
mod tests {
    use crate::vault::workspace::Workspace;
    use std::time::{Duration, SystemTime};

    #[test]
    fn expired_session_remaining_time_is_zero_and_locks_workspace() {
        let mut workspace = Workspace::new();
        workspace.start([3u8; 32]);
        workspace.session_start =
            Some(SystemTime::now() - Duration::from_secs(crate::vault::SESSION_TIMEOUT_SECS + 1));

        let remaining = super::session_remaining_seconds(&mut workspace);

        assert_eq!(remaining, 0);
        assert!(!workspace.is_unlocked());
    }
}
