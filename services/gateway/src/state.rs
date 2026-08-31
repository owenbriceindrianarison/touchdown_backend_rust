use shared::{config::AppConfig, nats::NatsRpc};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<AppConfig>,
    pub rpc: NatsRpc,
}
