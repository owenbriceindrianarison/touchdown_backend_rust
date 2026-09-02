pub mod application;
pub mod domain;
pub mod handlers;
pub mod infrastructure;
pub mod password;
pub mod state;
pub mod tokens;

use std::sync::Arc;

use contracts::auth::subjects;
use shared::{AppError, config::AppConfig, nats::RpcRouter};

use infrastructure::postgres::PgRepository;
pub use state::AuthState;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Builds the service RPC router.
///
/// Exposed publicly so integration tests mount the exact same router as the binary — not a diverging reimplementation.
pub fn build_router(cfg: &AppConfig, state: AuthState) -> RpcRouter {
    macro_rules! h {
        ($handler:path) => {{
            let state = state.clone();
            move |ctx, req| {
                let state = state.clone();
                async move { $handler(state, ctx, req).await }
            }
        }};
    }

    RpcRouter::new(cfg)
        .route(strip(subjects::REGISTER), h!(handlers::account::register))
        .route(strip(subjects::LOGIN), h!(handlers::account::login))
        .route(strip(subjects::REFRESH), h!(handlers::account::refresh))
        .route(strip(subjects::LOGOUT), h!(handlers::account::logout))
        .route(
            strip(subjects::REQUEST_RESET),
            h!(handlers::account::request_password_reset),
        )
        .route(
            strip(subjects::CONFIRM_RESET),
            h!(handlers::account::confirm_password_reset),
        )
        .route(
            strip(subjects::CONFIRM_EMAIL),
            h!(handlers::account::confirm_email),
        )
        .route(strip(subjects::GET_USER), h!(handlers::user::get))
        .route(strip(subjects::UPDATE_USER), h!(handlers::user::update))
        .route(
            strip(subjects::CHANGE_PASSWORD),
            h!(handlers::user::change_password),
        )
        .route(strip(subjects::LIST_USERS), h!(handlers::user::list))
        .route(strip(subjects::ADDRESS_LIST), h!(handlers::address::list))
        .route(strip(subjects::ADDRESS_GET), h!(handlers::address::get))
        .route(
            strip(subjects::ADDRESS_CREATE),
            h!(handlers::address::create),
        )
        .route(
            strip(subjects::ADDRESS_UPDATE),
            h!(handlers::address::update),
        )
        .route(
            strip(subjects::ADDRESS_SET_DEFAULT),
            h!(handlers::address::set_default),
        )
        .route(
            strip(subjects::ADDRESS_DELETE),
            h!(handlers::address::delete),
        )
}

/// Subjects in `contracts` are absolute (`auth.user.login`) while
/// `RpcRouter::route` expects a subject relative to the service.
fn strip(subject: &'static str) -> &'static str {
    subject.strip_prefix("auth.").unwrap_or(subject)
}

pub async fn init_state(cfg: &AppConfig) -> Result<AuthState, AppError> {
    let pool = shared::db::connect(cfg).await?;
    shared::db::run_migrations(&pool, &MIGRATOR).await?;
    Ok(AuthState {
        cfg: Arc::new(cfg.clone()),
        pool: pool.clone(),
        repo: Arc::new(PgRepository::new(pool)),
        issuer: Arc::new(shared::paseto::TokenIssuer::from_config(&cfg.paseto)?),
    })
}
