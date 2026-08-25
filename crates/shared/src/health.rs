use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{db::PgPool, nats::NatsRpc};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Down,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeathReport {
    pub service: String,
    pub version: String,
    pub status: HealthStatus,
    pub checks: Vec<HealthCheck>,
}

impl HeathReport {
    pub fn new(service: impl Into<String>, checks: Vec<HealthCheck>) -> Self {
        let status = if checks.iter().any(|c| c.status == HealthStatus::Down) {
            HealthStatus::Down
        } else if checks.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Ok
        };

        Self {
            service: service.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            status,
            checks,
        }
    }

    pub fn http_status(&self) -> u16 {
        match self.status {
            HealthStatus::Ok | HealthStatus::Degraded => 200,
            HealthStatus::Down => 503,
        }
    }
}

pub async fn check_postgres(pool: &PgPool) -> HealthCheck {
    let started = std::time::Instant::now();
    let (status, detail) = match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => (HealthStatus::Ok, None),
        Err(e) => (HealthStatus::Down, Some(e.to_string())),
    };

    HealthCheck {
        name: "postgres".into(),
        status,
        latency_ms: started.elapsed().as_millis() as u64,
        detail,
    }
}

pub async fn check_nats(rpc: &NatsRpc) -> HealthCheck {
    let started = std::time::Instant::now();
    let (status, detail) = if rpc.is_connected() {
        (HealthStatus::Ok, None)
    } else {
        (HealthStatus::Down, Some("not connected".into()))
    };

    HealthCheck {
        name: "nats".into(),
        status,
        latency_ms: started.elapsed().as_millis() as u64,
        detail,
    }
}
