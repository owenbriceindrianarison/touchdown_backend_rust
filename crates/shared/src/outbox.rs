use std::time::Duration;

use serde::Serialize;
use sqlx::{Postgres, Transaction, types::Json};
use uuid::Uuid;

use crate::{
    AppError,
    db::PgPool,
    nats::{EventEnvelope, JetStreamPublisher},
};

/// Inserts an event into the outbox WITHIN the business transaction.
///
/// This is the key to consistency: the data and its event are committed together.
/// Without this, a crash between the COMMIT and the NATS publish would leave a paid order that no one was notified about.
pub async fn enqueue<T: Serialize>(
    tx: &mut Transaction<'_, Postgres>,
    envelope: &EventEnvelope<T>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO outbox_events (id, subject, payload, trace_id)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(envelope.event_id)
    .bind(&envelope.event_type)
    .bind(Json(envelope))
    .bind(envelope.trace_id.as_deref())
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct OutboxRow {
    id: Uuid,
    subject: String,
    payload: serde_json::Value,
    trace_id: Option<String>,
}

/// Relay outbox -> JetStream. To be launched in a background task at startup.
///
/// Using `FOR UPDATE SKIP LOCKED`: multiple replicas of the same service
/// can run the relay in parallel without interfering with each other's rows.
pub struct OutboxRelay {
    pool: PgPool,
    publisher: JetStreamPublisher,
    batch_size: i64,
    poll_intervale: Duration,
}

impl OutboxRelay {
    pub fn new(pool: PgPool, publisher: JetStreamPublisher) -> Self {
        Self {
            pool,
            publisher,
            batch_size: 100,
            poll_intervale: Duration::from_millis(500),
        }
    }

    pub async fn run(self) {
        loop {
            match self.drain_once().await {
                Ok(0) => tokio::time::sleep(self.poll_intervale).await,
                Ok(n) => tracing::debug!(published = n, "outbox batch relayed"),
                Err(e) => {
                    tracing::error!(error = %e, "outbox relay failed");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    /// Claim a batch, publish, mark as published. Exposed publicly so that
    /// integration tests can trigger it without causing an infinite loop.
    pub async fn drain_once(&self) -> Result<usize, AppError> {
        // Claim: Increments the number of attempts and defers availability,
        // so that a failed publication is automatically retried.
        let rows: Vec<OutboxRow> = sqlx::query_as(
            r#"
            UPDATE outbox_events
                SET attempts = attempts + 1,
                    available_at = now() + interval '30 seconds'
                WHERE id IN (
                    SELECT if FROM outbox_events
                    WHERE published_at IS NULL AND available_at <= now()
                    ORDER BY available_at, id
                    LIMIT $1
                    FOR UPDATE SKIP LOCKED
                )
                RETURNING id, subject, payload, trace_id
            "#,
        )
        .bind(self.batch_size)
        .fetch_all(&self.pool)
        .await?;

        let mut published = Vec::with_capacity(rows.len());

        for row in &rows {
            let mut headers = async_nats::HeaderMap::new();
            headers.insert(crate::nats::HDR_MSG_ID, row.id.to_string().as_str());
            if let Some(tp) = &row.trace_id {
                headers.insert(crate::nats::HDR_TRACEPARENT, tp.as_str());
            }
            let body = serde_json::to_vec(&row.payload)?;

            match self
                .publisher
                .publish_raw(&row.subject, headers, body)
                .await
            {
                Ok(()) => published.push(row.id),
                Err(e) => {
                    let msg = e.to_string();
                    let _ = sqlx::query("UPDATE outbox_events SET last_error = $2 WHERE id = $1")
                        .bind(row.id)
                        .bind(&msg)
                        .execute(&self.pool)
                        .await;

                    tracing::warn!(id = %row.id, subject = %row.subject, error = %msg, "outbox publish failed");
                }
            }
        }

        if !published.is_empty() {
            sqlx::query("UPDATE outbox_events SET published_at = now() WHERE id =  ANY($1)")
                .bind(&published)
                .execute(&self.pool)
                .await?;
        }

        Ok(published.len())
    }
}

/// Marks an event as handled (inbox table). Returns `false` if it has already been viewed.
/// Must be called WITHIN the handler's transaction, before any side effects occur.
pub async fn mark_processed(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    consumer: &str,
    subject: &str,
) -> Result<bool, AppError> {
    let inserted = sqlx::query(
        r#"
        INSERT INTO processed_events (event_id, consumer, subject)
        VALUES ($1, $2, $3)
        ON CONFLICT (event_id, consumer) DO NOTHING
        "#,
    )
    .bind(event_id)
    .bind(consumer)
    .bind(subject)
    .execute(&mut **tx)
    .await?
    .rows_affected();

    Ok(inserted == 1)
}
