use std::time::SystemTime;

use crate::auth::authenticator::{AuthCredential, Authenticator};
use crate::auth::lockout::LockoutTracker;

use super::{storage::VaultStorage, workspace::Workspace};

pub struct VaultCoordinator {
    storage: VaultStorage,
    workspace: Workspace,
    lockout: LockoutTracker,
    schedule_session_expiry: Box<dyn Fn(SystemTime) + Send + Sync>,
}

impl VaultCoordinator {
    pub fn new(
        storage: VaultStorage,
        workspace: Workspace,
        schedule_session_expiry: Box<dyn Fn(SystemTime) + Send + Sync>,
    ) -> Self {
        Self {
            storage,
            workspace,
            lockout: LockoutTracker::new(),
            schedule_session_expiry,
        }
    }

    #[cfg(test)]
    fn with_lockout(
        storage: VaultStorage,
        workspace: Workspace,
        lockout: LockoutTracker,
        schedule_session_expiry: Box<dyn Fn(SystemTime) + Send + Sync>,
    ) -> Self {
        Self {
            storage,
            workspace,
            lockout,
            schedule_session_expiry,
        }
    }

    pub fn access(&mut self, credential: AuthCredential) -> Result<(), String> {
        if self.lockout.is_locked_out() {
            return Err("Too many failed attempts. Please try again later.".to_string());
        }

        let vault = self.storage.read()?;
        let key = Authenticator::derive_key(credential, &vault.kdf, &vault.salt)
            .map_err(|error| self.record_failed_access(error.to_string()))?;

        match super::access::access(&self.storage, &mut self.workspace, &key) {
            Ok(()) => {
                self.lockout.reset();
                if let Some(session_start) = self.workspace.session_start {
                    (self.schedule_session_expiry)(session_start);
                }
                Ok(())
            }
            Err(error) => Err(self.record_failed_access(error)),
        }
    }

    pub fn with_vault<F, T>(&mut self, operation: F) -> Result<T, String>
    where
        F: FnOnce(&VaultStorage, &mut Workspace) -> Result<T, String>,
    {
        operation(&self.storage, &mut self.workspace)
    }

    fn record_failed_access(&mut self, error: String) -> String {
        match self.lockout.record_failure() {
            Ok(()) => error,
            Err(lockout) => format!("{error}\n{lockout}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::VaultCoordinator;
    use crate::auth::authenticator::AuthCredential;
    use crate::auth::lockout::LockoutTracker;
    use crate::auth::method::AuthMethod;
    use crate::vault::storage::VaultStorage;
    use crate::vault::workspace::Workspace;

    fn password_vault(password: &str) -> (tempfile::TempDir, VaultStorage) {
        let directory = tempfile::tempdir().unwrap();
        let storage = VaultStorage {
            path: directory.path().join("vault.enc"),
        };
        let salt = [9_u8; 32];
        let key = crate::auth::password::derive_key(password, &salt);
        crate::vault::provision::provision(
            &storage,
            &mut Workspace::new(),
            &key,
            AuthMethod::Password,
            &hex::encode(salt),
        )
        .unwrap();
        (directory, storage)
    }

    fn coordinator_with_clock(storage: VaultStorage) -> (Arc<AtomicU64>, VaultCoordinator) {
        let elapsed = Arc::new(AtomicU64::new(0));
        let clock = Arc::clone(&elapsed);
        let start = Instant::now();
        let lockout = LockoutTracker::with_clock(Box::new(move || {
            start + Duration::from_secs(clock.load(Ordering::SeqCst))
        }));
        let coordinator =
            VaultCoordinator::with_lockout(storage, Workspace::new(), lockout, Box::new(|_| {}));
        (elapsed, coordinator)
    }

    #[test]
    fn failed_access_blocks_a_follow_up_attempt() {
        let (_directory, storage) = password_vault("correct");
        let mut coordinator = VaultCoordinator::new(storage, Workspace::new(), Box::new(|_| {}));

        assert!(coordinator
            .access(AuthCredential::Password("wrong".to_string()))
            .is_err());
        assert_eq!(
            coordinator
                .access(AuthCredential::Password("correct".to_string()))
                .unwrap_err(),
            "Too many failed attempts. Please try again later."
        );
    }

    #[test]
    fn successful_access_schedules_session_expiry() {
        let (_directory, storage) = password_vault("correct");
        let scheduled = Arc::new(AtomicBool::new(false));
        let callback_flag = Arc::clone(&scheduled);
        let mut coordinator = VaultCoordinator::new(
            storage,
            Workspace::new(),
            Box::new(move |_| callback_flag.store(true, Ordering::SeqCst)),
        );

        coordinator
            .access(AuthCredential::Password("correct".to_string()))
            .unwrap();

        assert!(scheduled.load(Ordering::SeqCst));
        assert!(coordinator
            .with_vault(|_, workspace| Ok(workspace.is_unlocked()))
            .unwrap());
    }

    #[test]
    fn failed_accesses_increase_the_lockout_delay() {
        let (_directory, storage) = password_vault("correct");
        let (elapsed, mut coordinator) = coordinator_with_clock(storage);

        let first = coordinator
            .access(AuthCredential::Password("wrong".to_string()))
            .unwrap_err();
        elapsed.store(6, Ordering::SeqCst);
        let second = coordinator
            .access(AuthCredential::Password("wrong".to_string()))
            .unwrap_err();

        assert!(first.contains("5 seconds"));
        assert!(second.contains("10 seconds"));
    }

    #[test]
    fn access_at_the_failure_threshold_stays_locked() {
        let (_directory, storage) = password_vault("correct");
        let (elapsed, mut coordinator) = coordinator_with_clock(storage);

        for attempt in 0..10 {
            coordinator
                .access(AuthCredential::Password("wrong".to_string()))
                .unwrap_err();
            if attempt < 9 {
                elapsed.store((attempt + 1) * 301, Ordering::SeqCst);
            }
        }

        assert_eq!(
            coordinator
                .access(AuthCredential::Password("correct".to_string()))
                .unwrap_err(),
            "Too many failed attempts. Please try again later."
        );
    }

    #[test]
    fn successful_access_resets_the_lockout_delay() {
        let (_directory, storage) = password_vault("correct");
        let (elapsed, mut coordinator) = coordinator_with_clock(storage);

        coordinator
            .access(AuthCredential::Password("wrong".to_string()))
            .unwrap_err();
        elapsed.store(6, Ordering::SeqCst);
        coordinator
            .access(AuthCredential::Password("correct".to_string()))
            .unwrap();
        let failure_after_success = coordinator
            .access(AuthCredential::Password("wrong".to_string()))
            .unwrap_err();

        assert!(failure_after_success.contains("5 seconds"));
    }
}
