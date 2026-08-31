use std::sync::Arc;

use axum::{Router, routing::get};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;

use shared::{config::AppConfig, nats::NatsRpc, telemetry};
use utoipa_swagger_ui::SwaggerUi;

use crate::{openapi::ApiDoc, state::AppState};

mod openapi;
mod routes {
    pub mod health;
}
mod state;

const SERVICE: &str = "gateway";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env(SERVICE)?;
    let _telemetry = telemetry::init(&cfg);

    let rpc = NatsRpc::connect(&cfg).await?;
    // The Gateway creates the streams at boot time: it starts before the other services,
    // so their consumers find a stream already in place.
    let publisher = shared::nats::JetStreamPublisher::new(rpc.client());
    shared::nats::ensure_streams(publisher.context()).await?;

    let state = AppState {
        cfg: Arc::new(cfg.clone()),
        rpc,
    };

    let cors = build_cors(&cfg);

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/readyz", get(routes::health::readyz))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(cfg.http_addr).await?;
    tracing::info!(addr = %cfg.http_addr, "gateway listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn build_cors(cfg: &AppConfig) -> CorsLayer {
    let origins = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default();
    let parsed: Vec<_> = origins
        .split(",")
        .filter(|s| !s.trim().is_empty())
        .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
        .collect();

    let layer = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    if parsed.is_empty() && cfg.environment.is_dev() {
        // Development only: Next/Angular frontends run on variable ports.
        // In production, unlisted origins are blocked.
        layer.allow_origin(tower_http::cors::Any)
    } else {
        layer
            .allow_origin(AllowOrigin::list(parsed))
            .allow_credentials(true)
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
