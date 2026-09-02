use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, rand_core::OsRng},
};
use shared::AppError;

/// Argon2id parameters aligned with OWASP recommendations:
/// 19 MiB of memory, 2 iterations, parallelism 1.
fn argon2() -> Argon2<'static> {
    let params = Params::new(19 * 1024, 2, 1, None).expect("valid argon2 params");
    Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params)
}

pub fn hash(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    argon2()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::internal(anyhow::anyhow!("argon2 hash: {e}")))
}

pub fn verify(password: &str, stored_hash: &str) -> bool {
    match PasswordHash::new(stored_hash) {
        Ok(parsed) => argon2()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// Dummy hash, verified when the email is unknown.
///
/// Without this, an instant response for an unknown email vs ~50 ms for a
/// known one turns the login endpoint into an account enumeration oracle.
pub fn dummy_verify(password: &str) {
    const DUMMY: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHR2YWx1ZQ$\
                         J8bKQhCXKMFHKZxKmMDdPqYHrxRLqPQ2yZ0Rm5Y1TgI";
    let _ = verify(password, DUMMY);
}
