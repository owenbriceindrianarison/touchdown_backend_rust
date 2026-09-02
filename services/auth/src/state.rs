use std::sync::Arc;

use shared::{config::AppConfig, db::PgPool, paseto::TokenIssuer};

use crate::domain::ports::AuthRepository;

#[derive(Clone)]
pub struct AuthState {
    pub cfg: Arc<AppConfig>,
    pub pool: PgPool,
    pub repo: Arc<dyn AuthRepository>,
    pub issuer: Arc<TokenIssuer>,
}

impl AuthState {
    pub const MAX_LOGIN_ATTEMPTS: i16 = 5;
    pub const LOCK_MINUTES: i64 = 15;
    pub const RESET_TOKEN_TTL_MINUTES: i64 = 60;
    pub const VERIFY_TOKEN_TTL_HOURS: i64 = 24;
}
