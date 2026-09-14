use crate::commands::VaultState;
use crate::vault::alias::AliasClient;
use crate::vault::workspace::Workspace;
use crate::vault::AliasConfig;
use serde_json::json;
use tauri::State;

fn configured_provider(
    workspace: &mut Workspace,
    provider_id: &str,
) -> Result<AliasConfig, String> {
    workspace.check_session()?;
    workspace.refresh();
    workspace
        .alias_configs
        .iter()
        .find(|config| config.provider_id == provider_id)
        .cloned()
        .ok_or_else(|| format!("Alias provider '{provider_id}' is not configured"))
}

#[tauri::command]
pub async fn save_alias_config(
    provider_id: String,
    api_token: String,
    description: Option<String>,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state.lock(|storage, workspace| {
        crate::vault::alias::save_config(
            workspace,
            storage,
            &provider_id,
            &api_token,
            description.as_deref(),
        )
    })?;

    Ok(json!({
        "status": "success"
    })
    .to_string())
}

fn alias_configs_payload(
    configs: Vec<crate::vault::alias::AliasProviderInfo>,
    default_provider_id: Option<String>,
) -> String {
    json!({
        "status": "success",
        "configs": configs,
        "default_provider_id": default_provider_id,
    })
    .to_string()
}

#[tauri::command]
pub async fn list_alias_configs(state: State<'_, VaultState>) -> Result<String, String> {
    let (configs, default_provider_id) = state.lock(|_, workspace| {
        let configs = crate::vault::alias::list_configs(workspace)?;
        Ok((configs, workspace.default_provider_id.clone()))
    })?;

    Ok(alias_configs_payload(configs, default_provider_id))
}

#[tauri::command]
pub async fn delete_alias_config(
    provider_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state.lock(|storage, workspace| {
        crate::vault::alias::delete_config(workspace, storage, &provider_id)
    })?;

    Ok(json!({ "status": "success" }).to_string())
}

#[tauri::command]
pub async fn set_default_alias_provider(
    provider_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state.lock(|storage, workspace| {
        crate::vault::alias::set_default_config(workspace, storage, &provider_id)
    })?;

    Ok(json!({ "status": "success" }).to_string())
}

#[tauri::command]
pub async fn generate_email_mask(
    provider_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let (config, mut cancel) = state.lock(|_, workspace| {
        let config = configured_provider(workspace, &provider_id)?;
        Ok((config, workspace.alias_cancel_receiver()))
    })?;

    let client = AliasClient::new()?;
    let email = tokio::select! {
        result = client.generate(&config) => result?,
        _ = cancel.changed() => return Err("Vault was locked".to_string()),
    };

    Ok(json!({
        "status": "success",
        "email": email
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn unlocked_workspace() -> Workspace {
        let mut workspace = Workspace::new();
        workspace.alias_configs.push(AliasConfig {
            provider_id: "simplelogin".to_string(),
            api_token: "sl-token".to_string(),
            description: None,
        });
        workspace.start([3u8; 32]);
        workspace
    }

    #[test]
    fn alias_configs_payload_exposes_providers_without_tokens() {
        let payload = alias_configs_payload(
            vec![crate::vault::alias::AliasProviderInfo {
                provider_id: "simplelogin".to_string(),
                description: Some("Work".to_string()),
            }],
            Some("simplelogin".to_string()),
        );

        assert!(payload.contains("\"provider_id\":\"simplelogin\""));
        assert!(payload.contains("\"description\":\"Work\""));
        assert!(payload.contains("\"default_provider_id\":\"simplelogin\""));
        assert!(!payload.contains("api_token"));
    }

    #[test]
    fn configured_provider_rejects_expired_sessions() {
        let mut workspace = unlocked_workspace();
        workspace.session_start =
            Some(SystemTime::now() - Duration::from_secs(crate::vault::SESSION_TIMEOUT_SECS + 1));

        let result = configured_provider(&mut workspace, "simplelogin");

        assert_eq!(result.unwrap_err(), "Session expired");
        assert!(workspace.alias_configs.is_empty());
    }

    #[test]
    fn configured_provider_rejects_unknown_providers() {
        let mut workspace = unlocked_workspace();

        let error = configured_provider(&mut workspace, "duckduckgo").unwrap_err();

        assert_eq!(error, "Alias provider 'duckduckgo' is not configured");
    }

    #[test]
    fn configured_provider_returns_the_stored_credentials() {
        let mut workspace = unlocked_workspace();

        let config = configured_provider(&mut workspace, "simplelogin").unwrap();

        assert_eq!(config.api_token, "sl-token");
    }
}
