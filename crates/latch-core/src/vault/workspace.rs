use super::{AliasConfig, Entry, SESSION_TIMEOUT_SECS};
use std::time::{Duration, SystemTime};
use tokio::sync::watch;
use zeroize::Zeroize;

pub struct Workspace {
    pub credentials: Vec<Entry>,
    pub alias_configs: Vec<AliasConfig>,
    pub default_provider_id: Option<String>,
    pub session_key: Option<zeroize::Zeroizing<[u8; 32]>>,
    pub session_start: Option<SystemTime>,
    session_generation: u64,
    alias_cancel: watch::Sender<bool>,
}

impl Workspace {
    pub fn new() -> Self {
        let (alias_cancel, _receiver) = watch::channel(false);
        Self {
            credentials: Vec::new(),
            alias_configs: Vec::new(),
            default_provider_id: None,
            session_key: None,
            session_start: None,
            session_generation: 0,
            alias_cancel,
        }
    }

    pub fn alias_cancel_receiver(&self) -> watch::Receiver<bool> {
        self.alias_cancel.subscribe()
    }

    pub fn is_unlocked(&self) -> bool {
        self.session_key.is_some()
    }

    pub fn check_session(&mut self) -> Result<(), String> {
        let generation = self.session_id().ok_or("Vault is locked")?;
        if self.expire_session(generation, SystemTime::now()) == Some(Duration::ZERO) {
            return Err("Session expired".to_string());
        }
        Ok(())
    }

    pub fn session_id(&self) -> Option<u64> {
        self.is_unlocked().then_some(self.session_generation)
    }

    /// Return the next idle deadline, locking at expiry or on an invalid clock.
    /// An old host timer cannot affect a later unlock or auth rotation.
    pub fn expire_session(&mut self, generation: u64, now: SystemTime) -> Option<Duration> {
        if self.session_id() != Some(generation) {
            return None;
        }
        let remaining = self
            .session_start
            .and_then(|start| now.duration_since(start).ok())
            .and_then(|elapsed| Duration::from_secs(SESSION_TIMEOUT_SECS).checked_sub(elapsed))
            .unwrap_or(Duration::ZERO);
        if remaining.is_zero() {
            self.lock();
        }
        Some(remaining)
    }

    pub fn refresh(&mut self) {
        self.session_start = Some(SystemTime::now());
    }

    pub fn lock(&mut self) {
        if let Some(ref mut key) = self.session_key {
            key.zeroize();
        }
        self.session_key = None;
        self.session_start = None;
        self.credentials.clear();
        self.alias_configs.clear();
        self.default_provider_id = None;
        self.alias_cancel.send_replace(true);
    }

    pub fn start(&mut self, key: [u8; 32]) {
        self.session_generation = self.session_generation.wrapping_add(1);
        self.session_key = Some(zeroize::Zeroizing::new(key));
        self.session_start = Some(SystemTime::now());
        self.alias_cancel.send_replace(false);
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn locking_the_vault_cancels_pending_alias_requests() {
        let mut workspace = Workspace::new();
        workspace.start([1u8; 32]);
        let mut cancel = workspace.alias_cancel_receiver();

        workspace.lock();

        let cancelled = tokio::select! {
            _ = std::future::pending::<()>() => false,
            _ = cancel.changed() => true,
        };
        assert!(cancelled);
    }

    #[test]
    fn unlocking_the_vault_resets_alias_request_cancellation() {
        let mut workspace = Workspace::new();
        workspace.lock();
        workspace.start([2u8; 32]);

        let cancel = workspace.alias_cancel_receiver();

        assert!(!*cancel.borrow());
    }

    #[test]
    fn idle_deadline_follows_activity_and_ignores_previous_sessions() {
        let mut workspace = Workspace::new();
        workspace.start([1; 32]);
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
        let first_session = workspace.session_id().unwrap();
        let start = workspace.session_start.unwrap();
        workspace.session_start = Some(start + Duration::from_secs(60));

        assert_eq!(
            workspace.expire_session(first_session, start + Duration::from_secs(1800)),
            Some(Duration::from_secs(60))
        );
        assert!(workspace.is_unlocked());
        assert_eq!(
            workspace.expire_session(first_session, start + Duration::from_secs(1860)),
            Some(Duration::ZERO)
        );
        assert!(workspace.session_key.is_none());
        assert!(workspace.credentials.is_empty());

        workspace.start([2; 32]);
        assert_eq!(
            workspace.expire_session(first_session, start + Duration::from_secs(3600)),
            None
        );
        assert!(workspace.is_unlocked());
    }

    #[test]
    fn invalid_session_clock_clears_the_key() {
        let mut workspace = Workspace::new();
        workspace.start([1; 32]);
        let generation = workspace.session_id().unwrap();
        let before_activity = workspace.session_start.unwrap() - Duration::from_secs(1);

        assert_eq!(
            workspace.expire_session(generation, before_activity),
            Some(Duration::ZERO)
        );
        assert!(!workspace.is_unlocked());
    }
}
