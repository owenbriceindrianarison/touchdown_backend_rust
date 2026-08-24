use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use async_nats::Client;
use futures::StreamExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    AppError, ErrorBody, config::AppConfig, locale::Locale, nats::envelope::RequestContext,
};

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
type BoxFut = Pin<Box<dyn Future<Output = Result<Vec<u8>, AppError>> + Send>>;
type Handler = Arc<dyn Fn(RequestContext, Vec<u8>) -> BoxFut + Send + Sync>;

/// Request-reply router for a service.
///
/// A SINGLE `<service>.>` subscription in a queue group, with internal dispatch
/// by topic: adding a route does not create an additional subscription,
/// and load balancing across replicas is handled by the queue group.
pub struct RpcRouter {
    service: String,
    routes: HashMap<String, Handler>,
    default_locale: Locale,
    concurrency: usize,
}

impl RpcRouter {
    pub fn new(cfg: &AppConfig) -> Self {
        Self {
            service: cfg.service_name.clone(),
            routes: HashMap::new(),
            default_locale: cfg.default_locale,
            concurrency: cfg.rpc_concurrency,
        }
        .route("health.check", |_ctx, _req: serde_json::Value| async move {
            Ok(serde_json::json!({ "status": "ok" }))
        })
    }

    /// `subject` refers to the service: `“user.login”` -> `auth.user.login`.
    pub fn route<Req, Res, F, Fut>(mut self, subject: &str, handler: F) -> Self
    where
        Req: DeserializeOwned + Send + 'static,
        Res: Serialize + Send + 'static,
        F: Fn(RequestContext, Req) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Res, AppError>> + Send + 'static,
    {
        let full = format!("{}.{}", self.service, subject);
        let handler = Arc::new(handler);
        let boxed: Handler = Arc::new(move |ctx, bytes| {
            let handler = handler.clone();
            Box::pin(async move {
                let req = serde_json::from_slice(&bytes)?;
                let res = handler(ctx, req).await?;
                serde_json::to_vec(&res).map_err(AppError::from)
            })
        });
        self.routes.insert(full, boxed);
        self
    }

    pub fn subjects(&self) -> Vec<&str> {
        self.routes.keys().map(String::as_str).collect()
    }

    /// Service loop. Does not return control until the subscriber stops.
    pub async fn run(self, rpc: NatsRpc) -> Result<(), AppError> {
        let wildcard = format!("{}.>", self.service);
        let mut sub = rpc
            .client()
            .queue_subscribe(wildcard.clone(), self.service.clone())
            .await
            .map_err(|e| AppError::Upstream(format!("cannot subscribe {wildcard}: {e}")))?;

        tracing::info!(subject = %wildcard, routes = self.routes.len(), "rpc router listening");

        let routes = Arc::new(self.routes);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let default_locale = self.default_locale;

        while let Some(msg) = sub.next().await {
            let Some(reply) = msg.reply.clone() else {
                tracing::warn!(subject = %&msg.subject, "rpc message without reply-to, dropped");
                continue;
            };

            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore closed");
            let routes = routes.clone();
            let client = rpc.client().clone();

            tokio::spawn(async move {
                let _permit = permit;
                let subject = msg.subject.to_string();
                let ctx = RequestContext::from_headers(msg.headers.as_ref(), default_locale);

                let outcome = match routes.get(&subject) {
                    Some(handler) => handler(ctx.clone(), msg.payload.to_vec()).await,
                    None => Err(AppError::not_found("subject", &subject)),
                };

                let body = match outcome {
                    Ok(data) => {
                        // `data` is already serialized JSON; we inject it as-is
                        // to avoid an extra round trip for deserialization.
                        let mut out = br#"{"status": "ok", "data":"#.to_vec();
                        out.extend_from_slice(&data);
                        out.push(b'}');
                        out
                    }
                    Err(err) => {
                        tracing::warn!(
                            subject = %subject,
                            request_id = %ctx.request_id,
                            code = err.code(),
                            error = %err,
                            "rpc handler failed"
                        );
                        let mut body = err.to_body();
                        body.trace_id = Some(ctx.request_id.clone());
                        serde_json::to_vec(&RpcResponse::<()>::Error {
                            status: err.status(),
                            error: body,
                        })
                        .unwrap_or_else(|_| br#"{"status":"error","status_code":500}"#.to_vec())
                    }
                };

                if let Err(e) = client.publish(reply, body.into()).await {
                    tracing::error!(subject = %subject, error = %e, "cannot publish rpc reply");
                }
            });
        }
        Ok(())
    }
}
