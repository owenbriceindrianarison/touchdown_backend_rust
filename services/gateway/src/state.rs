use shared::{config::AppConfig, nats::NatsRpc, paseto::TokenVerifier};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<AppConfig>,
    pub rpc: NatsRpc,
    pub verifier: Arc<TokenVerifier>,
}
