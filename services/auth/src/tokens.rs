use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// Opaque 256-bit token. This is not a PASETO: a refresh token does not need
/// to be readable, it only needs to be unpredictable and revocable.
pub fn generate() -> String {
    let bytes: [u8; 32] = rand::random();
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Only the hash is stored: a database leak yields no usable token.
pub fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
