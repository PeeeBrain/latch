pub mod credential;
pub mod generator;
pub mod health;
pub mod session;
pub mod vault;

use crate::auth::authenticator::AuthCredential;
use crate::vault::{coordinator::VaultCoordinator, storage::VaultStorage, workspace::Workspace};
use std::sync::{Arc, Mutex, Weak};
use tauri::AppHandle;

pub struct VaultState(pub Arc<Mutex<VaultCoordinator>>);

impl VaultState {
    pub fn new(storage: VaultStorage, workspace: Workspace, app_handle: AppHandle) -> Self {
        let coordinator = Arc::new_cyclic(|weak: &Weak<Mutex<VaultCoordinator>>| {
            let weak = weak.clone();
            Mutex::new(VaultCoordinator::new(
                storage,
                workspace,
                Box::new(move |session_start| {
                    crate::spawn_session_timer(app_handle.clone(), weak.clone(), session_start);
                }),
            ))
        });
        Self(coordinator)
    }

    pub fn lock<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&VaultStorage, &mut Workspace) -> Result<T, String>,
    {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| "Vault is temporarily unavailable")?;
        guard.with_vault(f)
    }

    pub fn access(&self, credential: AuthCredential) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|_| "Vault is temporarily unavailable")?
            .access(credential)
    }
}
