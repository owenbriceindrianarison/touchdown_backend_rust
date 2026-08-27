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

// #[cfg(feature = "testing")]
pub mod testing;

pub use error::{AppError, AppResult, ErrorBody};

pub mod prelude {
    pub use crate::config::{AppConfig, Environment};
    pub use crate::error::{AppError, AppResult, ErrorBody};
    pub use crate::ids::new_id;
    pub use crate::locale::Locale;
    pub use crate::nats::{EventEnvelope, JetStreamPublisher, NatsRpc, RequestContext, RpcRouter};
    pub use crate::pagination::{Page, PageParams};
}
