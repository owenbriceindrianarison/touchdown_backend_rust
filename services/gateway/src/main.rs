use std::sync::Arc;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::get,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use shared::{config::AppConfig, nats::NatsRpc, telemetry};

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

    // The Gateway creates the streams at boot time: it starts before the
    // other services, so their consumers find a stream already in place.
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

    let parsed: Vec<HeaderValue> = origins
        .split(',')
        .filter_map(|origin| {
            let origin = origin.trim();

            if origin.is_empty() {
                return None;
            }

            match origin.parse::<HeaderValue>() {
                Ok(value) => Some(value),
                Err(error) => {
                    tracing::warn!(
                        origin,
                        %error,
                        "ignoring invalid CORS origin"
                    );
                    None
                }
            }
        })
        .collect();

    let methods = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::PATCH,
        Method::DELETE,
        Method::OPTIONS,
    ];

    let headers = [header::ACCEPT, header::AUTHORIZATION, header::CONTENT_TYPE];

    let layer = CorsLayer::new()
        .allow_methods(methods)
        .allow_headers(headers);

    if parsed.is_empty() {
        if cfg.environment.is_dev() {
            tracing::debug!("CORS_ALLOWED_ORIGINS is not set; allowing any origin in development");

            // No credentials with wildcard origin.
            layer.allow_origin(tower_http::cors::Any)
        } else {
            tracing::warn!(
                "CORS_ALLOWED_ORIGINS is not set; no cross-origin requests will be allowed"
            );

            layer.allow_origin(AllowOrigin::list(Vec::<HeaderValue>::new()))
        }
    } else {
        tracing::debug!(
            origins = ?parsed,
            "configured CORS origins"
        );

        layer
            .allow_origin(AllowOrigin::list(parsed))
            .allow_credentials(true)
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
