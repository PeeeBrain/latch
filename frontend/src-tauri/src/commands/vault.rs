use crate::auth::authenticator::{AuthCredential, Authenticator};
use crate::commands::VaultState;
use serde_json::json;
use tauri::State;

#[tauri::command]
pub async fn init_vault_oauth(
    id_token: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let user_id = crate::auth::oauth::extract_user_id(&id_token)
        .map_err(|e| format!("Invalid ID token: {}", e))?;
    let key = Authenticator::derive_key(AuthCredential::OAuthToken(id_token), "oauth-argon2id", "")
        .map_err(|error| error.to_string())?;

    state.lock(|storage, workspace| {
        crate::vault::provision::provision(
            storage,
            workspace,
            &key,
            crate::auth::method::AuthMethod::OAuth,
            &user_id,
        )
    })?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn init_vault_with_key(
    key_hex: String,
    kdf: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let auth_method = crate::auth::method::AuthMethod::from_vault_tag(&kdf)
        .ok_or_else(|| format!("Unknown KDF: {}", kdf))?;
    let key = Authenticator::derive_key(AuthCredential::RawKeyHex(key_hex), &kdf, "")
        .map_err(|error| error.to_string())?;

    state.lock(|storage, workspace| {
        crate::vault::provision::provision(storage, workspace, &key, auth_method, "")
    })?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn init_vault(password: String, state: State<'_, VaultState>) -> Result<String, String> {
    let salt = crate::auth::password::generate_salt();
    let salt_hex = hex::encode(salt);
    let key = Authenticator::derive_key(
        AuthCredential::Password(password),
        "password-pbkdf2",
        &salt_hex,
    )
    .map_err(|error| error.to_string())?;

    state.lock(|storage, workspace| {
        crate::vault::provision::provision(
            storage,
            workspace,
            &key,
            crate::auth::method::AuthMethod::Password,
            &salt_hex,
        )
    })?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn unlock_vault_oauth(
    id_token: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state.access(AuthCredential::OAuthToken(id_token))?;
    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn unlock_vault_with_key(
    key_hex: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state.access(AuthCredential::RawKeyHex(key_hex))?;
    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn unlock_vault(
    password: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    state.access(AuthCredential::Password(password))?;
    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn get_vault_auth_method(state: State<'_, VaultState>) -> Result<String, String> {
    state.lock(|storage, _| {
        let method = if storage.exists() {
            storage
                .read()
                .map(|v| v.kdf)
                .unwrap_or_else(|_| "none".to_string())
        } else {
            "none".to_string()
        };

        Ok(json!({
            "status": "success",
            "auth_method": method
        })
        .to_string())
    })
}

#[tauri::command]
pub async fn vault_status(state: State<'_, VaultState>) -> Result<String, String> {
    state.lock(|storage, workspace| {
        let unlocked = workspace.is_unlocked();
        let has_vault = storage.exists();
        Ok(json!({
            "status": "success",
            "has_vault": has_vault,
            "is_unlocked": unlocked
        })
        .to_string())
    })
}

#[tauri::command]
pub async fn reencrypt_vault(
    new_key_hex: String,
    new_kdf: String,
    new_salt: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let auth_method = crate::auth::method::AuthMethod::from_vault_tag(&new_kdf)
        .ok_or_else(|| format!("Unknown KDF: {}", new_kdf))?;
    let key = Authenticator::derive_key(AuthCredential::RawKeyHex(new_key_hex), &new_kdf, "")
        .map_err(|error| error.to_string())?;

    state.lock(|storage, workspace| {
        crate::vault::rotate::rotate(storage, workspace, &key, auth_method, &new_salt)
    })?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn reencrypt_vault_to_oauth(
    id_token: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let user_id = crate::auth::oauth::extract_user_id(&id_token)
        .map_err(|e| format!("Invalid ID token: {}", e))?;
    let key = Authenticator::derive_key(AuthCredential::OAuthToken(id_token), "oauth-argon2id", "")
        .map_err(|error| error.to_string())?;

    state.lock(|storage, workspace| {
        crate::vault::rotate::rotate(
            storage,
            workspace,
            &key,
            crate::auth::method::AuthMethod::OAuth,
            &user_id,
        )
    })?;

    Ok(json!({"status": "success"}).to_string())
}

#[tauri::command]
pub async fn migrate_to_oauth(
    password: String,
    id_token: String,
    state: State<'_, VaultState>,
) -> Result<String, String> {
    let user_id = crate::auth::oauth::extract_user_id(&id_token)
        .map_err(|e| format!("Invalid ID token: {}", e))?;

    state.lock(|storage, workspace| {
        let vault_file = storage.read()?;
        if vault_file.kdf != "password-pbkdf2" {
            return Err("Migration is only supported from password-based vaults".to_string());
        }

        let password_key = Authenticator::derive_key(
            AuthCredential::Password(password),
            &vault_file.kdf,
            &vault_file.salt,
        )
        .map_err(|error| error.to_string())?;
        crate::vault::access::access(storage, workspace, &password_key)?;

        let oauth_key =
            Authenticator::derive_key(AuthCredential::OAuthToken(id_token), "oauth-argon2id", "")
                .map_err(|error| error.to_string())?;
        crate::vault::rotate::rotate(
            storage,
            workspace,
            &oauth_key,
            crate::auth::method::AuthMethod::OAuth,
            &user_id,
        )
    })?;

    Ok(json!({"status": "success"}).to_string())
}
