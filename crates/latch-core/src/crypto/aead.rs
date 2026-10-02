use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedData {
    pub nonce: String,
    pub ciphertext: String,
}

pub fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<EncryptedData, String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(EncryptedData {
        nonce: hex::encode(nonce),
        ciphertext: hex::encode(ciphertext),
    })
}

pub fn decrypt(key: &[u8; 32], data: &EncryptedData) -> Result<String, String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce_bytes =
        hex::decode(&data.nonce).map_err(|e| format!("Invalid nonce encoding: {}", e))?;
    if nonce_bytes.len() != 12 {
        return Err("Invalid vault nonce length".into());
    }
    let ciphertext =
        hex::decode(&data.ciphertext).map_err(|e| format!("Invalid ciphertext encoding: {}", e))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("Invalid UTF-8 in decrypted data: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = "hello world";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let encrypted = encrypt(&key1, "secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn test_decrypt_tampered_ciphertext_fails() {
        let key = [1u8; 32];
        let mut encrypted = encrypt(&key, "secret").unwrap();
        encrypted.ciphertext = "deadbeef".to_string();
        assert!(decrypt(&key, &encrypted).is_err());
        encrypted.nonce = "00".into();
        assert!(decrypt(&key, &encrypted).is_err());
    }
}
