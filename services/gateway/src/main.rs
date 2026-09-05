use std::sync::Arc;

use gateway::{AppState, app};
use shared::{config::AppConfig, nats::NatsRpc, paseto::TokenVerifier, telemetry};

const SERVICE: &str = "gateway";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env(SERVICE)?;
    let _telemetry = telemetry::init(&cfg);

    let rpc = NatsRpc::connect(&cfg).await?;
    let publisher = shared::nats::JetStreamPublisher::new(rpc.client());
    shared::nats::ensure_streams(publisher.context()).await?;

    let state = AppState {
        verifier: Arc::new(TokenVerifier::from_config(&cfg.paseto)?),
        cfg: Arc::new(cfg.clone()),
        rpc,
    };

    let listener = tokio::net::TcpListener::bind(cfg.http_addr).await?;
    tracing::info!(addr = %cfg.http_addr, "gateway listening");

    axum::serve(listener, app(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
