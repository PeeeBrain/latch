use super::{storage::VaultStorage, workspace::Workspace, EncryptedVault, VaultData};
use crate::auth::method::AuthMethod;
use crate::crypto::aead;

pub fn provision(
    storage: &VaultStorage,
    workspace: &mut Workspace,
    key: &[u8; 32],
    method: AuthMethod,
    salt: &str,
) -> Result<(), String> {
    if storage.exists() {
        return Err("Vault already exists".to_string());
    }

    let vault_data = VaultData {
        entries: Vec::new(),
        alias_configs: Vec::new(),
        default_provider_id: None,
    };
    let json = serde_json::to_string(&vault_data)
        .map_err(|e| format!("Failed to serialize vault data: {}", e))?;

    let encrypted = aead::encrypt(key, &json)?;

    let vault = EncryptedVault {
        version: "2".to_string(),
        kdf: method.vault_tag().to_string(),
        salt: salt.to_string(),
        data: encrypted,
    };

    storage.write(&vault)?;
    workspace.start(*key);

    Ok(())
}
