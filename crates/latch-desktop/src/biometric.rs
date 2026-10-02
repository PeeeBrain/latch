use zeroize::Zeroizing;

#[cfg(any(target_os = "windows", target_os = "macos"))]
const DOMAIN: &str = "com.latch.vault";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const NAME: &str = "vault-encryption-key";

pub fn supported() -> bool {
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

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    pub fn create_or_retrieve() -> Result<zeroize::Zeroizing<String>, String> {
        retrieve()
    }
    pub fn retrieve() -> Result<zeroize::Zeroizing<String>, String> {
        Err("Biometric device keys are unavailable on this platform. Use a password vault.".into())
    }
}
