use std::time::Duration;

use async_nats::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{AppError, ErrorBody, config::AppConfig, nats::envelope::RequestContext};

/// RPC response format. The error is passed along the bus with its original status.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RpcResponse<T> {
    Ok { data: T },
    Error { status: u16, error: ErrorBody },
}

// ========= client =========

#[derive(Clone)]
pub struct NatsRpc {
    client: Client,
    timeout: Duration,
}

impl NatsRpc {
    pub async fn connect(cfg: &AppConfig) -> Result<Self, AppError> {
        let client = async_nats::ConnectOptions::new()
            .name(cfg.service_name.clone())
            .retry_on_initial_connect()
            .connect(&cfg.nats_url)
            .await
            .map_err(|e| AppError::Config(format!("cannot connect to NATS: {e}")))?;

        tracing::info!(url = %cfg.nats_url, "nats connected");
        Ok(Self {
            client,
            timeout: cfg.nats_request_timeout,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn is_connected(&self) -> bool {
        matches!(
            self.client.connection_state(),
            async_nats::connection::State::Connected
        )
    }

    /// Synchronous service->service call.
    #[tracing::instrument(skip(self, req), fields(subject = %subject))]
    pub async fn request<Req, Res>(
        &self,
        subject: &str,
        ctx: &RequestContext,
        req: &Req,
    ) -> Result<Res, AppError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        let body = serde_json::to_vec(req).map_err(AppError::from)?;
        let msg = tokio::time::timeout(
            self.timeout,
            self.client
                .request_with_headers(subject.to_string(), ctx.to_headers(), body.into()),
        )
        .await
        .map_err(|_| AppError::Timeout(subject.to_string()))?
        .map_err(|e| AppError::Upstream(format!("nats request failed on {subject}: {e}")))?;

        match serde_json::from_slice::<RpcResponse<Res>>(&msg.payload) {
            Ok(RpcResponse::Ok { data }) => Ok(data),
            Ok(RpcResponse::Error { status, error }) => Err(AppError::Remote {
                status,
                body: error,
            }),
            Err(e) => Err(AppError::Upstream(format!(
                "malformed reply on {subject}: {e}"
            ))),
        }
    }
}

// ======================== Server ======================
