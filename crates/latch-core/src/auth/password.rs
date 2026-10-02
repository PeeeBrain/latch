use pbkdf2::pbkdf2_hmac;
use rand::Rng;
use sha2::Sha256;

const PBKDF2_ITERATIONS: u32 = 100_000;

pub fn derive_key(password: &str, salt: &[u8; 32]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

pub fn generate_salt() -> [u8; 32] {
    rand::thread_rng().gen()
}
