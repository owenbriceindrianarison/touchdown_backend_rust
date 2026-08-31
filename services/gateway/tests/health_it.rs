//! Gateway level test: the HTTP server starts up, connects to a real NATS,
//! and /readyz reflects the status of that connection.

use shared::{config::AppConfig, nats::NatsRpc};

#[tokio::test]
async fn health_and_readyz_respond() {
    let nats = shared::testing::nats().await;

    let mut cfg = AppConfig::from_env("gateway").unwrap();
    cfg.nats_url = nats.url.clone();
    cfg.http_addr = "127.0.0.1:0".parse().unwrap();

    let rpc = NatsRpc::connect(&cfg).await.unwrap();
    let listener = tokio::net::TcpListener::bind(cfg.http_addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Minimal router identical to the one in the main function.
    let state = gateway_test_state(cfg, rpc);
    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/readyz", axum::routing::get(readyz_handler))
        .with_state(state);

    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let health = reqwest::get(format!("http://{addr}/health")).await.unwrap();
    assert_eq!(health.status(), 200);
    let body: serde_json::Value = health.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "gateway");

    let ready = reqwest::get(format!("http://{addr}/readyz")).await.unwrap();
    assert_eq!(ready.status(), 200);
    let body: serde_json::Value = ready.json().await.unwrap();
    assert_eq!(body["checks"][0]["name"], "nats");
    assert_eq!(body["checks"][0]["status"], "ok");
}

#[derive(Clone)]
struct TestState {
    cfg: std::sync::Arc<AppConfig>,
    rpc: NatsRpc,
}

fn gateway_test_state(cfg: AppConfig, rpc: NatsRpc) -> TestState {
    TestState {
        cfg: std::sync::Arc::new(cfg),
        rpc,
    }
}

async fn health_handler(
    axum::extract::State(state): axum::extract::State<TestState>,
) -> axum::Json<shared::health::HealthReport> {
    axum::Json(shared::health::HealthReport::new(
        state.cfg.service_name.clone(),
        vec![shared::health::HealthCheck {
            name: "process".into(),
            status: shared::health::HealthStatus::Ok,
            latency_ms: 0,
            detail: None,
        }],
    ))
}

async fn readyz_handler(
    axum::extract::State(state): axum::extract::State<TestState>,
) -> axum::Json<shared::health::HealthReport> {
    axum::Json(shared::health::HealthReport::new(
        state.cfg.service_name.clone(),
        vec![shared::health::check_nats(&state.rpc).await],
    ))
}
