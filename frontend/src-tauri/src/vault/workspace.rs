use super::{AliasConfig, Entry, SESSION_TIMEOUT_SECS};
use std::time::SystemTime;
use zeroize::Zeroize;

pub struct Workspace {
    pub credentials: Vec<Entry>,
    pub alias_configs: Vec<AliasConfig>,
    pub default_provider_id: Option<String>,
    pub session_key: Option<zeroize::Zeroizing<[u8; 32]>>,
    pub session_start: Option<SystemTime>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            credentials: Vec::new(),
            alias_configs: Vec::new(),
            default_provider_id: None,
            session_key: None,
            session_start: None,
        }
    }

    pub fn is_unlocked(&self) -> bool {
        self.session_key.is_some()
    }

    pub fn check_session(&mut self) -> Result<(), String> {
        if self.session_key.is_none() {
            return Err("Vault is locked".to_string());
        }
        if let Some(start) = self.session_start {
            let elapsed = start
                .elapsed()
                .map_err(|e| format!("Failed to get elapsed time: {}", e))?
                .as_secs();
            if elapsed > SESSION_TIMEOUT_SECS {
                self.lock();
                return Err("Session expired".to_string());
            }
        } else {
            return Err("Invalid session".to_string());
        }
        Ok(())
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
    }

    pub fn start(&mut self, key: [u8; 32]) {
        self.session_key = Some(zeroize::Zeroizing::new(key));
        self.session_start = Some(SystemTime::now());
    }
}
