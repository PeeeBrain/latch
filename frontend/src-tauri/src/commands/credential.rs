use crate::commands::VaultState;
use crate::vault::Entry;
use serde_json::json;
use tauri::State;

/// Build the editable-payload JSON for a credential. The raw `totp_secret` is
/// never serialized to the webview; only its presence is exposed.
fn serialize_full_entry(entry: &Entry) -> String {
    json!({
        "status": "success",
        "entry": {
            "id": entry.id,
            "title": entry.title,
            "username": entry.username,
            "password": entry.password,
            "url": entry.url,
            "icon_url": entry.icon_url,
            "has_totp": entry.totp_secret.is_some(),
        }
    })
    .to_string()
}

fn normalize_totp_secret(totp_secret: Option<String>) -> Result<Option<String>, String> {
    match totp_secret {
        Some(value) if !value.trim().is_empty() => {
            Ok(Some(crate::vault::totp::extract_secret(&value)?))
        }
        _ => Ok(None),
    }
}

fn validate_entry_fields(
    title: &str,
    username: &str,
    password: &str,
    url: Option<&String>,
) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("Title cannot be empty".to_string());
    }
    if title.len() > 256 {
        return Err("Title is too long (max 256 characters)".to_string());
    }

    if username.trim().is_empty() {
        return Err("Username cannot be empty".to_string());
    }
    if username.len() > 256 {
        return Err("Username is too long (max 256 characters)".to_string());
    }

    if password.trim().is_empty() {
        return Err("Password cannot be empty".to_string());
    }
    if password.len() > 1024 {
        return Err("Password is too long (max 1024 characters)".to_string());
    }

    if let Some(url_val) = url {
        if !url_val.trim().is_empty() {
            match url::Url::parse(url_val) {
                Ok(parsed) => {
                    let scheme = parsed.scheme();
                    if scheme != "http" && scheme != "https" {
                        return Err("URL must use http or https scheme".to_string());
                    }
                }
                Err(e) => return Err(format!("Invalid URL: {}", e)),
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn search_entries(query: String, state: State<'_, VaultState>) -> Result<String, String> {
    let results = state.lock(|_, workspace| crate::vault::search::search(workspace, &query))?;
    Ok(json!({
        "status": "success",
        "entries": results
    })
    .to_string())
}

#[tauri::command]
pub async fn request_secret(
    entry_id: String,
    field: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let secret = state
        .lock(|_, workspace| crate::vault::entries::get_field(workspace, &entry_id, &field))?;

    Ok(json!({"status": "success", "value": secret}).to_string())
}

#[tauri::command]
pub async fn add_entry(
    title: String,
    username: String,
    password: String,
    url: Option<String>,
    icon_url: Option<String>,
    totp_secret: Option<String>,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    validate_entry_fields(&title, &username, &password, url.as_ref())?;

    let id = uuid::Uuid::new_v4().to_string();
    let entry = crate::vault::Entry {
        id: id.clone(),
        title,
        username,
        password,
        url,
        icon_url,
        totp_secret: normalize_totp_secret(totp_secret)?,
    };

    state.lock(|storage, workspace| crate::vault::entries::add(workspace, storage, entry))?;

    Ok(json!({"status": "success", "id": id}).to_string())
}

#[tauri::command]
pub async fn get_full_entry(
    entry_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let entry = state.lock(|_, workspace| crate::vault::entries::get_full(workspace, &entry_id))?;

    Ok(serialize_full_entry(&entry))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_entry(
    id: String,
    title: String,
    username: String,
    password: String,
    url: Option<String>,
    icon_url: Option<String>,
    totp_secret: Option<String>,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    validate_entry_fields(&title, &username, &password, url.as_ref())?;

    let entry = crate::vault::Entry {
        id,
        title,
        username,
        password,
        url,
        icon_url,
        totp_secret: normalize_totp_secret(totp_secret)?,
    };

    state.lock(|storage, workspace| crate::vault::entries::update(workspace, storage, entry))?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn delete_entry(
    entry_id: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state
        .lock(|storage, workspace| crate::vault::entries::delete(workspace, storage, &entry_id))?;

    Ok(json!({"status": "success"}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_with_secret() -> Entry {
        Entry {
            id: "entry-1".to_string(),
            title: "Example".to_string(),
            username: "user".to_string(),
            password: "secret".to_string(),
            url: Some("https://example.com".to_string()),
            icon_url: None,
            totp_secret: Some("GEZDGNBVGY3TQOJQ".to_string()),
        }
    }

    #[test]
    fn full_entry_payload_never_exposes_totp_secret() {
        let payload = serialize_full_entry(&entry_with_secret());

        assert!(!payload.contains("GEZDGNBVGY3TQOJQ"));
        assert!(!payload.contains("totp_secret"));
        assert!(payload.contains("\"has_totp\":true"));
    }

    #[test]
    fn full_entry_payload_keeps_editable_fields() {
        let payload = serialize_full_entry(&entry_with_secret());

        assert!(payload.contains("\"title\":\"Example\""));
        assert!(payload.contains("\"password\":\"secret\""));
        assert!(payload.contains("\"status\":\"success\""));
    }

    #[test]
    fn normalize_totp_secret_clears_blank_and_extracts_uris() {
        assert_eq!(normalize_totp_secret(None).unwrap(), None);
        assert_eq!(normalize_totp_secret(Some("   ".into())).unwrap(), None);
        assert_eq!(
            normalize_totp_secret(Some(" gezd gnbv ".into())).unwrap(),
            Some("GEZDGNBV".to_string())
        );
        assert!(normalize_totp_secret(Some("otpauth://totp/A?secret=GEZD1NBV".into())).is_err());
    }
}
