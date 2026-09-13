use super::{storage::VaultStorage, workspace::Workspace, EncryptedVault, VaultData};
use crate::auth::method::AuthMethod;
use crate::crypto::aead;

pub fn rotate(
    storage: &VaultStorage,
    workspace: &mut Workspace,
    new_key: &[u8; 32],
    new_method: AuthMethod,
    new_salt: &str,
) -> Result<(), String> {
    workspace.check_session()?;

    let vault_data = VaultData::from_workspace(workspace);
    let json = serde_json::to_string(&vault_data)
        .map_err(|e| format!("Failed to serialize vault data: {}", e))?;
    let encrypted = aead::encrypt(new_key, &json)?;

    let vault = EncryptedVault {
        version: "2".to_string(),
        kdf: new_method.vault_tag().to_string(),
        salt: new_salt.to_string(),
        data: encrypted,
    };

    storage.write(&vault)?;
    workspace.start(*new_key);

    Ok(())
}
