/// Launch argument the login entry passes; main starts the window dismissed.
pub const MINIMIZED_ARG: &str = "--minimized";

/// Whether a per-user login entry exists.
pub fn enabled() -> bool {
    platform::enabled()
}

/// Registers or removes the current executable's per-user login entry.
pub fn set(enabled: bool) -> Result<(), String> {
    // Run from a DMG or a quarantined download, macOS hands Latch a path that
    // is gone after a reboot, so the entry would silently do nothing.
    #[cfg(all(not(test), target_os = "macos"))]
    if enabled
        && std::env::current_exe().is_ok_and(|exe| {
            let exe = exe.to_string_lossy();
            exe.contains("/AppTranslocation/") || exe.starts_with("/Volumes/")
        })
    {
        return Err("Move Latch to Applications first".into());
    }
    platform::set(enabled).map_err(|_| "Couldn't update start at login".to_string())
}

/// Tests (and the verify-latch drive) keep the entry in memory, so they never
/// register the test binary with the OS.
#[cfg(test)]
mod platform {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ENTRY: AtomicBool = AtomicBool::new(false);

    pub fn enabled() -> bool {
        ENTRY.load(Ordering::SeqCst)
    }

    pub fn set(enabled: bool) -> Result<(), std::convert::Infallible> {
        ENTRY.store(enabled, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(all(not(test), any(target_os = "linux", target_os = "macos")))]
mod platform {
    use std::{io::Write, path::PathBuf};

    fn entry_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .or_else(|| {
                    std::env::var_os("HOME")
                        .filter(|home| !home.is_empty())
                        .map(|home| PathBuf::from(home).join(".config"))
                })?;
            Some(config.join("autostart/com.latch.passwordmanager.desktop"))
        }
        #[cfg(target_os = "macos")]
        {
            Some(
                PathBuf::from(std::env::var_os("HOME").filter(|home| !home.is_empty())?)
                    .join("Library/LaunchAgents/com.latch.passwordmanager.plist"),
            )
        }
    }

    pub fn enabled() -> bool {
        entry_path().is_some_and(|path| path.exists())
    }

    pub fn set(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
        let path = entry_path().ok_or("Missing home directory")?;
        if !enabled {
            return match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            };
        }
        #[cfg(target_os = "linux")]
        let executable = std::env::var_os("APPIMAGE")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .map_or_else(std::env::current_exe, Ok)?;
        #[cfg(target_os = "macos")]
        let executable = std::env::current_exe()?;
        if !executable.is_absolute() {
            return Err("Executable path must be absolute".into());
        }
        let executable = executable.to_str().ok_or("Invalid executable path")?;
        if executable.chars().any(char::is_control) {
            return Err("Invalid executable path".into());
        }
        #[cfg(target_os = "linux")]
        let contents = format!(
            "[Desktop Entry]\nType=Application\nName=Latch\nExec={} {}\nX-GNOME-Autostart-enabled=true\nNoDisplay=true\n",
            super::desktop_exec(executable)?,
            super::MINIMIZED_ARG,
        );
        #[cfg(target_os = "macos")]
        let contents = {
            let executable = executable
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;");
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                 <plist version=\"1.0\"><dict>\n\
                 <key>Label</key><string>com.latch.passwordmanager</string>\n\
                 <key>ProgramArguments</key><array><string>{executable}</string><string>{}</string></array>\n\
                 <key>RunAtLoad</key><true/>\n\
                 </dict></plist>\n",
                super::MINIMIZED_ARG,
            )
        };
        let directory = path.parent().ok_or("Invalid login entry path")?;
        std::fs::create_dir_all(directory)?;
        // A same-directory rename keeps an interrupted write from truncating the entry.
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        temporary.write_all(contents.as_bytes())?;
        temporary.persist(path)?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn desktop_exec(path: &str) -> Result<String, &'static str> {
    // Desktop Exec paths cannot contain '='; escaping has both string and quoting layers.
    if path.contains('=') || path.chars().any(char::is_control) {
        return Err("Invalid executable path");
    }
    let mut quoted = String::from("\"");
    for character in path.chars() {
        match character {
            '\\' => quoted.push_str(r"\\\\"),
            '"' | '`' | '$' => {
                quoted.push_str(r"\\");
                quoted.push(character);
            }
            '%' => quoted.push_str("%%"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    Ok(quoted)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn desktop_exec_quotes_literal_path() {
        assert_eq!(
            super::desktop_exec("/tmp/Latch space/\"$`\\%f.AppImage").unwrap(),
            r#""/tmp/Latch space/\\"\\$\\`\\\\%%f.AppImage""#,
        );
        assert!(super::desktop_exec("/tmp/a=b").is_err());
        assert!(super::desktop_exec("/tmp/Latch\nExec=other").is_err());
    }
}

#[cfg(all(not(test), target_os = "windows"))]
mod platform {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows::{
        Win32::{
            Foundation::ERROR_FILE_NOT_FOUND,
            System::Registry::{
                HKEY, HKEY_CURRENT_USER, REG_ROUTINE_FLAGS, REG_SZ, RRF_RT_REG_BINARY,
                RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyW, RegDeleteKeyValueW, RegGetValueW,
                RegSetValueExW,
            },
        },
        core::{PCWSTR, w},
    };

    const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    /// Startup apps (Settings, Task Manager) record their on/off switch here, and
    /// Windows skips a Run entry whose first byte is odd. No value means on.
    const APPROVED_KEY: PCWSTR =
        w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");

    fn read(key: PCWSTR, kind: REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
        let mut size = 0u32;
        // SAFETY: Static, terminated names; the first call only asks for the size,
        // the second writes at most `size` bytes into a buffer of that length.
        unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key,
                w!("Latch"),
                kind,
                None,
                None,
                Some(&mut size),
            )
            .ok()
            .ok()?;
            let mut data = vec![0u8; size as usize];
            RegGetValueW(
                HKEY_CURRENT_USER,
                key,
                w!("Latch"),
                kind,
                None,
                Some(data.as_mut_ptr().cast()),
                Some(&mut size),
            )
            .ok()
            .ok()?;
            data.truncate(size as usize);
            Some(data)
        }
    }

    /// Whether Windows will start Latch at sign-in. A release build also re-points
    /// an entry whose executable is gone, so a moved install keeps starting.
    pub fn enabled() -> bool {
        let Some(command) = read(RUN_KEY, RRF_RT_REG_SZ) else {
            return false;
        };
        if read(APPROVED_KEY, RRF_RT_REG_BINARY)
            .is_some_and(|flags| flags.first().is_some_and(|flag| flag & 1 == 1))
        {
            return false;
        }
        // Our command is `"<exe>" --minimized`; the path sits between the first two quotes.
        let wide: Vec<u16> = command
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes(*unit))
            .collect();
        let missing = wide
            .split(|&unit| unit == u16::from(b'"'))
            .nth(1)
            .is_some_and(|path| {
                !std::path::Path::new(&std::ffi::OsString::from_wide(path)).exists()
            });
        if missing && !cfg!(debug_assertions) {
            let _ = set(true);
        }
        true
    }

    pub fn set(enabled: bool) -> Result<(), windows::core::Error> {
        // Latch's own toggle wins over an earlier Startup apps "off".
        // SAFETY: The predefined handle and static, terminated names are valid.
        let _ = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, APPROVED_KEY, w!("Latch")) };
        if !enabled {
            // SAFETY: The predefined handle and static, terminated names are valid.
            let result = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, w!("Latch")) };
            return if result == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                result.ok()
            };
        }
        let executable = std::env::current_exe().map_err(|_| windows::core::Error::from_win32())?;
        let command: Vec<u16> = std::iter::once(b'"' as u16)
            .chain(executable.as_os_str().encode_wide())
            .chain(format!("\" {}\0", super::MINIMIZED_ARG).encode_utf16())
            .collect();
        let bytes: Vec<u8> = command.iter().flat_map(|unit| unit.to_le_bytes()).collect();
        let mut key = HKEY::default();
        // SAFETY: Names are terminated, the output handle is valid, and bytes includes a UTF-16 NUL.
        unsafe {
            RegCreateKeyW(HKEY_CURRENT_USER, RUN_KEY, &mut key).ok()?;
            let result = RegSetValueExW(key, w!("Latch"), None, REG_SZ, Some(&bytes));
            let _ = RegCloseKey(key);
            result.ok()
        }
    }
}
