use std::fs;
use std::io::Write;
use std::path::PathBuf;

use super::EncryptedVault;

pub struct VaultStorage {
    pub path: PathBuf,
}

impl VaultStorage {
    pub fn new() -> Result<Self, String> {
        let path = get_vault_path()?;
        let config_dir = path.parent().ok_or("Invalid vault path")?;
        fs::create_dir_all(config_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
        Ok(Self { path })
    }

    /// Keep this handle alive for the host process to exclude another current host.
    pub fn acquire_process_lock(&self) -> Result<fs::File, String> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.path.with_extension("lock"))
            .map_err(|e| format!("Cannot open vault lock: {e}"))?;
        file.try_lock().map_err(|_| {
            "Another Latch process is using this vault. Close it before continuing.".to_string()
        })?;
        Ok(file)
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn read(&self) -> Result<EncryptedVault, String> {
        self.inspect()?
            .ok_or_else(|| "Vault does not exist".to_string())
    }

    /// Inspect without treating corrupt or inaccessible files as a new vault.
    pub fn inspect(&self) -> Result<Option<EncryptedVault>, String> {
        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("Failed to read vault: {error}")),
        };
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|error| format!("Failed to parse vault: {error}"))
    }

    pub fn write(&self, vault: &EncryptedVault) -> Result<(), String> {
        let json = serde_json::to_string_pretty(vault)
            .map_err(|e| format!("Failed to serialize vault: {}", e))?;

        let directory = self.path.parent().ok_or("Invalid vault directory")?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)
            .map_err(|e| format!("Failed to create vault temporary file: {e}"))?;
        temporary
            .write_all(json.as_bytes())
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|e| format!("Failed to write vault: {e}"))?;
        temporary
            .persist(&self.path)
            .map_err(|e| format!("Failed to replace vault: {e}"))?;
        Ok(())
    }
}

fn get_vault_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .map(|p| {
            if cfg!(target_os = "linux") {
                p.join("latch")
            } else {
                p.join("Latch")
            }
        })
        .ok_or("Failed to get config dir")?;
    Ok(config_dir.join("vault.enc"))
}

#[cfg(test)]
mod tests {
    use super::VaultStorage;
    use std::fs;

    #[test]
    fn inspection_distinguishes_missing_and_invalid_vaults_without_writing() {
        let directory = tempfile::tempdir().unwrap();
        let storage = VaultStorage {
            path: directory.path().join("vault.enc"),
        };
        assert!(storage.inspect().unwrap().is_none());
        assert!(!storage.path.exists());
        let first = storage.acquire_process_lock().unwrap();
        assert!(storage.acquire_process_lock().is_err());
        drop(first);
        assert!(storage.acquire_process_lock().is_ok());

        let invalid = b"not a vault";
        fs::write(&storage.path, invalid).unwrap();
        assert!(storage.inspect().is_err());
        assert_eq!(fs::read(&storage.path).unwrap(), invalid);

        // A directory exercises a read failure on every supported OS.
        let unreadable = VaultStorage {
            path: directory.path().to_path_buf(),
        };
        assert!(unreadable.inspect().is_err());
    }
}
