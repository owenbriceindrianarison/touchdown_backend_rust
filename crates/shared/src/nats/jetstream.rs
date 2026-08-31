use std::time::Duration;

use async_nats::{
    HeaderMap,
    jetstream::{self, Context, consumer::pull::Config as PullConfig},
};
use futures::StreamExt;
use serde::{Serialize, de::DeserializeOwned};

use super::envelope::{EventEnvelope, HDR_MSG_ID, HDR_TRACEPARENT};
use crate::error::AppError;

#[derive(Clone)]
pub struct JetStreamPublisher {
    ctx: Context,
}

impl JetStreamPublisher {
    pub fn new(client: &async_nats::Client) -> Self {
        Self {
            ctx: jetstream::new(client.clone()),
        }
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Publishes and WAITS for the server's acknowledgment (double `await`). Without the second`await`,
    /// we wouldn't know if the message was actually persisted,
    /// and the relay outbox would incorrectly mark events as published.
    pub async fn publish<T: Serialize>(&self, envelope: &EventEnvelope<T>) -> Result<(), AppError> {
        let mut headers = HeaderMap::new();
        headers.insert(HDR_MSG_ID, envelope.event_id.to_string().as_str());
        if let Some(tp) = &envelope.trace_id {
            headers.insert(HDR_TRACEPARENT, tp.as_str());
        }
        let payload = serde_json::to_vec(envelope)?;
        self.publish_raw(&envelope.event_type, headers, payload)
            .await
    }

    pub async fn publish_raw(
        &self,
        subject: &str,
        headers: HeaderMap,
        payload: Vec<u8>,
    ) -> Result<(), AppError> {
        let ack = self
            .ctx
            .publish_with_headers(subject.to_string(), headers, payload.into())
            .await
            .map_err(|e| AppError::Upstream(format!("jetstream publish {subject}: {e}")))?;
        ack.await
            .map_err(|e| AppError::Upstream(format!("jetstream ack {subject}: {e}")))?;
        Ok(())
    }
}

pub struct ConsumerSpec {
    pub stream: &'static str,
    pub durable: String,
    pub filter_subjects: Vec<String>,
    pub max_deliver: i64,
    pub ack_wait: Duration,
}

/// Sustainable consumption loop with DLQ.
///
/// After `max_deliver` failures, the message is sent to `dlq.<durable>.<topic>` and
/// is acknowledged: it must not block the consumer indefinitely. Resending
/// is done from the DLQ stream, either manually or via an admin job.
pub async fn run_consumer<T, F, Fut>(
    js: &Context,
    spec: ConsumerSpec,
    handler: F,
) -> Result<(), AppError>
where
    T: DeserializeOwned + Send + 'static,
    F: Fn(EventEnvelope<T>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), AppError>> + Send,
{
    let stream = js
        .get_stream(spec.stream)
        .await
        .map_err(|e| AppError::Upstream(format!("stream {}: {e}", spec.stream)))?;

    let consumer = stream
        .get_or_create_consumer(
            &spec.durable,
            PullConfig {
                durable_name: Some(spec.durable.clone()),
                filter_subjects: spec.filter_subjects.clone(),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                ack_wait: spec.ack_wait,
                max_deliver: spec.max_deliver,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| AppError::Upstream(format!("consumer {}: {e}", spec.durable)))?;

    let mut messages = consumer
        .messages()
        .await
        .map_err(|e| AppError::Upstream(format!("consumer stream: {e}")))?;

    tracing::info!(stream = spec.stream, durable = %spec.durable, "jetstream consumer started");

    while let Some(Ok(msg)) = messages.next().await {
        let subject = msg.subject.to_string();
        let delivered = msg.info().map(|i| i.delivered).unwrap_or(1);

        match serde_json::from_slice::<EventEnvelope<T>>(&msg.payload) {
            Ok(envelope) => match handler(envelope).await {
                Ok(()) => {
                    let _ = msg.ack().await;
                }
                Err(e) if delivered >= spec.max_deliver => {
                    tracing::error!(subject = %subject, error = %e, "max deliveries reached, sending to DLQ");
                    let dlq_subject = format!("dlq.{}.{}", spec.durable, subject);
                    let _ = js.publish(dlq_subject, msg.payload.clone()).await;
                    let _ = msg.ack().await;
                }
                Err(e) => {
                    tracing::warn!(subject = %subject, attempt = delivered, error = %e, "handler failed, will retry");
                    // Exponential backoff limited to 60 seconds.
                    let delay =
                        Duration::from_secs(1u64 << delivered.min(6)).min(Duration::from_secs(60));
                    let _ = msg.ack_with(jetstream::AckKind::Nak(Some(delay))).await;
                }
            },
            Err(e) => {
                // An unreadable payload will never become readable: direct to DLQ.
                tracing::error!(subject = %subject, error = %e, "undecodable event, sending to DLQ");
                let _ = js
                    .publish(
                        format!("dlq.{}.{}", spec.durable, subject),
                        msg.payload.clone(),
                    )
                    .await;
                let _ = msg.ack().await;
            }
        }
    }

    Ok(())
}
