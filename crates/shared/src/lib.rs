pub mod config;
pub mod db;
pub mod error;
pub mod health;
pub mod ids;
pub mod locale;
pub mod nats;
pub mod openapi;
pub mod outbox;
pub mod pagination;
pub mod paseto;
pub mod telemetry;

pub use error::{AppError, AppResult, ErrorBody};
