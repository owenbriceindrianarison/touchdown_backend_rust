use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use shared::health::{HealthCheck, HealthReport, check_nats};

use crate::state::AppState;

/// Liveness. Does NOT test ANY dependencies: if this endpoint fails, the process is dead and the orchestrator must restart it.
/// Adding a Postgres check here would cause the pod to be terminated every time the database experiences an issue.
#[utoipa::path(
    get, path = "/health", tag = "system",
    responses((status = 200, description = "The process responds", body = HealthReport))
)]
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let report = HealthReport::new(
        state.cfg.service_name.clone(),
        vec![HealthCheck {
            name: "process".into(),
            status: shared::health::HealthStatus::Ok,
            latency_ms: 0,
            detail: None,
        }],
    );

    (StatusCode::OK, Json(report))
}

/// Readiness. Tests dependencies: as long as NATS is unreachable,
/// the Gateway must not receive any traffic.
#[utoipa::path(
    get, path = "readyz", tag = "system",
    responses(
        (status = 200, description = "Ready to serve", body = HealthReport),
    (status = 503, description = "Dependency Unavailable", body = HealthReport)
    )
)]
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let report = HealthReport::new(
        state.cfg.service_name.clone(),
        vec![check_nats(&state.rpc).await],
    );
    let status = StatusCode::from_u16(report.http_status()).unwrap();

    (status, Json(report))
}
