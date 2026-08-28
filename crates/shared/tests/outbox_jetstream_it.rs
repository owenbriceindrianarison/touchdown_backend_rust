use std::time::Duration;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use shared::{
    nats::{Actor, EventEnvelope},
    outbox::{OutboxRelay, enqueue},
};

#[derive(Debug, Serialize, Deserialize)]
struct OrderCreated {
    order_id: String,
    total: i64,
}

const OUTBOX_DDL: &str = r#"
CREATE TABLE outbox_events (
    id           uuid PRIMARY KEY,
    subject      text        NOT NULL,
    payload      jsonb       NOT NULL,
    headers      jsonb       NOT NULL DEFAULT '{}'::jsonb,
    trace_id     text,
    attempts     smallint    NOT NULL DEFAULT 0,
    last_error   text,
    available_at timestamptz NOT NULL DEFAULT now(),
    published_at timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);
"#;

#[tokio::test]
async fn outbox_relays_to_jetstream_and_dedupes() {
    // 1. Infrastructure
    let pg = shared::testing::postgres(None).await;
    let nats = shared::testing::nats().await;

    sqlx::query(OUTBOX_DDL).execute(&pg.pool).await.unwrap();

    // 2. Business Transaction + Outbox
    let envelope = EventEnvelope::new(
        "events.order.created",
        OrderCreated {
            order_id: "TD-2026-000001".into(),
            total: 19_900,
        },
        Actor::system(),
    );

    let event_id = envelope.event_id;
    let mut tx = pg.pool.begin().await.unwrap();
    enqueue(&mut tx, &envelope).await.unwrap();
    tx.commit().await.unwrap();

    // 3. Relay : PostgreSQL -> JetStream
    let relay = OutboxRelay::new(pg.pool.clone(), nats.publisher.clone());

    assert_eq!(relay.drain_once().await.unwrap(), 1);
    assert_eq!(relay.drain_once().await.unwrap(), 0);

    let published: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT published_at
             FROM outbox_events
             WHERE id = $1",
    )
    .bind(event_id)
    .fetch_one(&pg.pool)
    .await
    .unwrap();

    assert!(published.is_some());

    // 4. Sustainable consumption
    let js = nats.publisher.context();
    let mut stream = js.get_stream("ORDERS").await.unwrap();

    let consumer = stream
        .get_or_create_consumer(
            "test-consumer",
            async_nats::jetstream::consumer::pull::Config {
                durable_name: Some("test-consumer".into()),
                filter_subjects: vec!["events.order.>".into()],
                ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let mut messages = consumer
        .fetch()
        .max_messages(1)
        .expires(Duration::from_secs(2))
        .messages()
        .await
        .unwrap();

    let message = tokio::time::timeout(Duration::from_secs(5), messages.next())
        .await
        .expect("timeout waiting for message")
        .expect("no message received")
        .expect("failed to receive message");

    let received: EventEnvelope<OrderCreated> = serde_json::from_slice(&message.payload).unwrap();
    message.ack().await.unwrap();

    assert_eq!(received.event_id, event_id);
    assert_eq!(received.event_type, "events.order.created");
    assert_eq!(received.payload.order_id, "TD-2026-000001");

    // 5. Repost the same event

    nats.publisher.publish(&envelope).await.unwrap();

    // The stream must always contain only one message.
    let info = stream.info().await.unwrap();

    assert_eq!(
        info.state.messages, 1,
        "Nats-Msg-Id should deduplicate the event"
    );
}
