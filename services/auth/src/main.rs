use shared::{
    config::AppConfig,
    nats::{JetStreamPublisher, NatsRpc},
    outbox::OutboxRelay,
    telemetry,
};

const SERVICE: &str = "auth";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env(SERVICE)?;
    let _telemetry = telemetry::init(&cfg);

    let state = auth::init_state(&cfg).await?;
    let rpc = NatsRpc::connect(&cfg).await?;
    let publisher = JetStreamPublisher::new(rpc.client());
    shared::nats::ensure_streams(publisher.context()).await?;

    // Outbox relay: publishes to JetStream events written in the same transaction as the business data.
    tokio::spawn(OutboxRelay::new(state.pool.clone(), publisher).run());

    let router = auth::build_router(&cfg, state);
    tracing::info!(routes = ?router.subjects(), "auth service starting");

    tokio::select! {
        res = router.run(rpc) => res?,
        _ = tokio::signal::ctrl_c() => tracing::info!("shutdown signal received"),
    }

    Ok(())
}
