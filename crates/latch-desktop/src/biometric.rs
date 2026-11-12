use zeroize::Zeroizing;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
const DOMAIN: &str = "com.latch.vault";
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
const NAME: &str = "vault-encryption-key";

pub fn supported() -> bool {
    #[cfg(target_os = "linux")]
    return std::path::Path::new(platform::POLICY).exists();
    #[cfg(not(target_os = "linux"))]
    cfg!(any(target_os = "windows", target_os = "macos"))
}

pub fn retrieve() -> Result<Zeroizing<String>, String> {
    let value = platform::retrieve().map_err(|error| format!("Device key unavailable: {error}"))?;
    let decoded =
        Zeroizing::new(hex::decode(value.as_str()).map_err(|_| "Invalid stored device key")?);
    if decoded.len() != 32 {
        return Err("Invalid stored device key".into());
    }
    Ok(value)
}

pub fn create_or_retrieve() -> Result<Zeroizing<String>, String> {
    platform::create_or_retrieve().map_err(|error| format!("Device key unavailable: {error}"))
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::{
        Security::{
            Credentials::{
                KeyCredentialCreationOption, KeyCredentialManager, KeyCredentialStatus,
                PasswordCredential, PasswordVault,
            },
            Cryptography::{
                BinaryStringEncoding,
                Core::{
                    CryptographicEngine, HashAlgorithmNames, HashAlgorithmProvider,
                    SymmetricAlgorithmNames, SymmetricKeyAlgorithmProvider,
                },
                CryptographicBuffer,
            },
        },
        core::*,
    };

    fn wrapping_key(
        create: bool,
    ) -> Result<(
        windows::Security::Cryptography::Core::CryptographicKey,
        windows::Storage::Streams::IBuffer,
    )> {
        // Preserve the existing plugin's domain, account, signed challenge, SHA-256
        // wrapping key and CBC encoding. Retrieving the ciphertext alone is insufficient.
        let mut result = KeyCredentialManager::OpenAsync(&HSTRING::from(DOMAIN))?.get()?;
        if create && result.Status()? == KeyCredentialStatus::NotFound {
            raise_security_prompt();
            result = KeyCredentialManager::RequestCreateAsync(
                &HSTRING::from(DOMAIN),
                KeyCredentialCreationOption::FailIfExists,
            )?
            .get()?;
        }
        if result.Status()? != KeyCredentialStatus::Success {
            return Err(Error::from(HRESULT(0x80070490u32 as i32)));
        }
        let credential = result.Credential()?;
        let challenge = CryptographicBuffer::ConvertStringToBinary(
            &HSTRING::from(DOMAIN),
            BinaryStringEncoding::Utf8,
        )?;
        raise_security_prompt();
        let signature = credential.RequestSignAsync(&challenge)?.get()?;
        if signature.Status()? != KeyCredentialStatus::Success {
            return Err(Error::from(HRESULT(0x800704c7u32 as i32)));
        }
        let hash = HashAlgorithmProvider::OpenAlgorithm(&HashAlgorithmNames::Sha256()?)?;
        let wrapping = hash.HashData(&signature.Result()?)?;
        let iv_hash = hash.HashData(&CryptographicBuffer::ConvertStringToBinary(
            &HSTRING::from(format!("IV_{DOMAIN}")),
            BinaryStringEncoding::Utf8,
        )?)?;
        let mut iv_bytes = Array::<u8>::new();
        CryptographicBuffer::CopyToByteArray(&iv_hash, &mut iv_bytes)?;
        let iv = CryptographicBuffer::CreateFromByteArray(&iv_bytes[..16])?;
        let aes =
            SymmetricKeyAlgorithmProvider::OpenAlgorithm(&SymmetricAlgorithmNames::AesCbcPkcs7()?)?;
        let key = aes.CreateSymmetricKey(&wrapping)?;
        Ok((key, iv))
    }

    /// KeyCredentialManager has no owner-window API, so Windows opens the Hello
    /// prompt behind Latch. Latch is foreground when the user asks to unlock,
    /// which lets it hand foreground to the prompt once the prompt appears.
    // ponytail: matches the prompt by window class; revisit if Windows renames it.
    fn raise_security_prompt() {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow};
        std::thread::spawn(|| {
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Ok(prompt) = unsafe { FindWindowW(w!("Credential Dialog Xaml Host"), None) }
                {
                    let _ = unsafe { SetForegroundWindow(prompt) };
                    return;
                }
            }
        });
    }

    pub fn create_or_retrieve() -> Result<Zeroizing<String>> {
        let vault = PasswordVault::new()?;
        // Never replace a key that could still protect an existing vault.
        match vault.Retrieve(&HSTRING::from(DOMAIN), &HSTRING::from(NAME)) {
            Ok(_) => return retrieve(),
            Err(error) if error.code() == HRESULT(0x80070490u32 as i32) => {}
            Err(error) => return Err(error),
        }
        let (key, iv) = wrapping_key(true)?;
        let random = Zeroizing::new(latch_core::auth::password::generate_salt());
        let value = Zeroizing::new(hex::encode(random.as_ref()));
        let plain = CryptographicBuffer::CreateFromByteArray(value.as_bytes())?;
        let encrypted = CryptographicEngine::Encrypt(&key, &plain, Some(&iv))?;
        let encoded = CryptographicBuffer::EncodeToBase64String(&encrypted)?;
        let stored = PasswordCredential::CreatePasswordCredential(
            &HSTRING::from(DOMAIN),
            &HSTRING::from(NAME),
            &encoded,
        )?;
        vault.Add(&stored)?;
        let verified = retrieve()?;
        if verified.as_str() != value.as_str() {
            return Err(Error::from(HRESULT(0x8007000du32 as i32)));
        }
        Ok(verified)
    }

    pub fn retrieve() -> Result<Zeroizing<String>> {
        let (key, iv) = wrapping_key(false)?;
        let stored =
            PasswordVault::new()?.Retrieve(&HSTRING::from(DOMAIN), &HSTRING::from(NAME))?;
        stored.RetrievePassword()?;
        let ciphertext = CryptographicBuffer::DecodeFromBase64String(&stored.Password()?)?;
        let decrypted = CryptographicEngine::Decrypt(&key, &ciphertext, Some(&iv))?;
        let mut bytes = Array::<u8>::new();
        CryptographicBuffer::CopyToByteArray(&decrypted, &mut bytes)?;
        let copy = Zeroizing::new(bytes.to_vec());
        bytes.fill(0);
        String::from_utf8(copy.to_vec())
            .map(Zeroizing::new)
            .map_err(|_| Error::from(HRESULT(0x8007000du32 as i32)))
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2_core_foundation::{CFBoolean, CFData, CFDictionary, CFRetained, CFString, CFType};
    use objc2_security::{
        SecItemCopyMatching, kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword,
        kSecMatchLimit, kSecMatchLimitOne, kSecReturnData, kSecUseDataProtectionKeychain,
    };
    pub fn create_or_retrieve() -> Result<Zeroizing<String>, String> {
        let random = Zeroizing::new(latch_core::auth::password::generate_salt());
        let value = Zeroizing::new(hex::encode(random.as_ref()));
        let data = CFData::from_bytes(value.as_bytes());
        let account = CFString::from_str(NAME);
        let service = CFString::from_str(DOMAIN);
        let yes = CFBoolean::new(true);
        // SAFETY: Security constants are valid CF objects; no error output is requested.
        let access = unsafe {
            objc2_security::SecAccessControl::with_flags(
                None,
                objc2_security::kSecAttrAccessibleWhenUnlockedThisDeviceOnly.as_ref(),
                objc2_security::SecAccessControlCreateFlags::UserPresence,
                std::ptr::null_mut(),
            )
        }
        .ok_or("Cannot create user-presence Keychain protection")?;
        // SAFETY: from_slices retains all values through SecItemAdd.
        let attributes = unsafe {
            CFDictionary::<CFType, CFType>::from_slices(
                &[
                    kSecClass.as_ref(),
                    kSecAttrAccount.as_ref(),
                    kSecAttrService.as_ref(),
                    objc2_security::kSecValueData.as_ref(),
                    objc2_security::kSecAttrAccessControl.as_ref(),
                    kSecUseDataProtectionKeychain.as_ref(),
                ],
                &[
                    kSecClassGenericPassword.as_ref(),
                    account.as_ref(),
                    service.as_ref(),
                    data.as_ref(),
                    access.as_ref(),
                    yes.as_ref(),
                ],
            )
        };
        // SAFETY: attributes outlive the call; a null result pointer requests no output.
        let status =
            unsafe { objc2_security::SecItemAdd(attributes.as_opaque(), std::ptr::null_mut()) };
        if status == objc2_security::errSecDuplicateItem {
            return retrieve();
        }
        if status != 0 {
            return Err(format!("Keychain enrollment error {status}"));
        }
        let verified = retrieve()?;
        if verified.as_str() != value.as_str() {
            return Err("Stored device key verification failed".into());
        }
        Ok(verified)
    }
    // Keep the existing Keychain prompt contract until a signed upgrade proves LAContext continuity.
    #[allow(deprecated)]
    pub fn retrieve() -> Result<Zeroizing<String>, String> {
        use objc2_security::kSecUseOperationPrompt;
        let account = CFString::from_str(NAME);
        let service = CFString::from_str(DOMAIN);
        let reason = CFString::from_str("Unlock Latch");
        let yes = CFBoolean::new(true);
        // SAFETY: constant keys are valid CF objects. from_slices retains every
        // value; account/service/reason remain alive throughout the Security call.
        let query = unsafe {
            CFDictionary::<CFType, CFType>::from_slices(
                &[
                    kSecClass.as_ref(),
                    kSecAttrAccount.as_ref(),
                    kSecAttrService.as_ref(),
                    kSecReturnData.as_ref(),
                    kSecMatchLimit.as_ref(),
                    kSecUseOperationPrompt.as_ref(),
                    kSecUseDataProtectionKeychain.as_ref(),
                ],
                &[
                    kSecClassGenericPassword.as_ref(),
                    account.as_ref(),
                    service.as_ref(),
                    yes.as_ref(),
                    kSecMatchLimitOne.as_ref(),
                    reason.as_ref(),
                    yes.as_ref(),
                ],
            )
        };
        let mut output = std::ptr::null();
        // SAFETY: the retained dictionary outlives the call; output is a writable
        // pointer. On success the Copy rule transfers one ownership reference.
        let status = unsafe { SecItemCopyMatching(query.as_opaque(), &mut output) };
        if status != 0 {
            return Err(format!("Keychain error {status}"));
        }
        let pointer = std::ptr::NonNull::new(output.cast_mut()).ok_or("Empty Keychain result")?;
        let retained = unsafe { CFRetained::<CFType>::from_raw(pointer) };
        let data = retained
            .downcast::<CFData>()
            .map_err(|_| "Invalid Keychain result")?;
        let value =
            String::from_utf8(data.to_vec()).map_err(|_| "Invalid Keychain key encoding")?;
        Ok(Zeroizing::new(value))
    }
}

/// Linux has no OS store that releases a key only after a fingerprint, so this is
/// a presence gate: polkit runs the user's PAM stack (fingerprint, falling back to
/// the login password), then the key is read from the login keyring through the
/// Secret Service. See docs/adr/0003-linux-device-key.md for the trust model.
#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use oo7::{Keyring, zbus};
    use std::collections::HashMap;

    /// Installed by the deb; AppImage users install it by hand (README).
    pub const POLICY: &str = "/usr/share/polkit-1/actions/com.latch.vault.policy";
    const ACTION: &str = "com.latch.vault.unlock";
    /// polkit `CheckAuthorizationFlags::AllowUserInteraction`: show the auth prompt.
    const ALLOW_USER_INTERACTION: u32 = 1;
    const ATTRIBUTES: [(&str, &str); 2] = [("service", DOMAIN), ("account", NAME)];

    async fn authorize() -> Result<(), String> {
        let unavailable = |_| "polkit is unavailable".to_string();
        let bus = zbus::Connection::system().await.map_err(unavailable)?;
        let name = bus
            .unique_name()
            .ok_or("polkit is unavailable")?
            .to_string();
        // A bus-name subject lets polkit resolve this process itself, without PID races.
        let subject = (
            "system-bus-name",
            HashMap::from([("name", zbus::zvariant::Value::from(name))]),
        );
        let reply = bus
            .call_method(
                Some("org.freedesktop.PolicyKit1"),
                "/org/freedesktop/PolicyKit1/Authority",
                Some("org.freedesktop.PolicyKit1.Authority"),
                "CheckAuthorization",
                &(
                    subject,
                    ACTION,
                    HashMap::<&str, &str>::new(),
                    ALLOW_USER_INTERACTION,
                    "",
                ),
            )
            .await
            .map_err(unavailable)?;
        let (authorized, _, _): (bool, bool, HashMap<String, String>) =
            reply.body().deserialize().map_err(unavailable)?;
        if authorized {
            Ok(())
        } else {
            Err("authentication was cancelled".into())
        }
    }

    async fn keyring() -> Result<Keyring, String> {
        let keyring = Keyring::new().await.map_err(|error| error.to_string())?;
        keyring.unlock().await.map_err(|error| error.to_string())?;
        Ok(keyring)
    }

    async fn stored(keyring: &Keyring) -> Result<Option<Zeroizing<String>>, String> {
        let items = keyring
            .search_items(&ATTRIBUTES)
            .await
            .map_err(|error| error.to_string())?;
        let Some(item) = items.into_iter().next() else {
            return Ok(None);
        };
        item.unlock().await.map_err(|error| error.to_string())?;
        // Secret zeroizes on drop; the str borrow avoids an unzeroized copy.
        let secret = item.secret().await.map_err(|error| error.to_string())?;
        let value =
            std::str::from_utf8(secret.as_bytes()).map_err(|_| "Invalid stored device key")?;
        Ok(Some(Zeroizing::new(value.to_owned())))
    }

    /// Every keyring access goes through here, so the key is never read without
    /// polkit approval. Futures are lazy: `keyring` doesn't start until `gate` passes.
    async fn gated<T>(
        gate: impl Future<Output = Result<(), String>>,
        keyring: impl Future<Output = Result<T, String>>,
    ) -> Result<T, String> {
        gate.await?;
        keyring.await
    }

    // zbus::block_on is the executor zbus' own blocking API uses. Callers already
    // run on a background thread, like the blocking Windows and macOS calls.
    pub fn create_or_retrieve() -> Result<Zeroizing<String>, String> {
        zbus::block_on(gated(authorize(), async {
            let keyring = keyring().await?;
            // Never replace a key that could still protect an existing vault.
            if let Some(existing) = stored(&keyring).await? {
                return Ok(existing);
            }
            let random = Zeroizing::new(latch_core::auth::password::generate_salt());
            let value = Zeroizing::new(hex::encode(random.as_ref()));
            keyring
                .create_item("Latch vault key", &ATTRIBUTES, value.as_bytes(), false)
                .await
                .map_err(|error| error.to_string())?;
            let verified = stored(&keyring)
                .await?
                .ok_or("Stored device key verification failed")?;
            if verified.as_str() != value.as_str() {
                return Err("Stored device key verification failed".into());
            }
            Ok(verified)
        }))
    }

    pub fn retrieve() -> Result<Zeroizing<String>, String> {
        zbus::block_on(gated(authorize(), async {
            stored(&keyring().await?)
                .await?
                .ok_or_else(|| "no device key is stored".into())
        }))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;

        #[test]
        fn denied_authorization_never_touches_the_keyring() {
            let touched = Cell::new(false);
            let result = zbus::block_on(gated(
                async { Err::<(), _>("authentication was cancelled".to_string()) },
                async {
                    touched.set(true);
                    Ok(Zeroizing::new("key".to_string()))
                },
            ));
            assert_eq!(result.unwrap_err(), "authentication was cancelled");
            assert!(!touched.get());
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod platform {
    pub fn create_or_retrieve() -> Result<zeroize::Zeroizing<String>, String> {
        retrieve()
    }
    pub fn retrieve() -> Result<zeroize::Zeroizing<String>, String> {
        Err("Biometric device keys are unavailable on this platform. Use a password vault.".into())
    }
}
