use crate::commands::VaultState;
use serde_json::json;
use tauri::State;

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
        totp_secret,
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

    Ok(json!({
        "status": "success",
        "entry": {
            "id": entry.id,
            "title": entry.title,
            "username": entry.username,
            "password": entry.password,
            "url": entry.url,
            "icon_url": entry.icon_url,
        }
    })
    .to_string())
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
        totp_secret,
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
