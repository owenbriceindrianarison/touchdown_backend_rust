use async_nats::jetstream::{Context, stream};

use crate::AppError;

pub struct StreamSpec {
    pub name: &'static str,
    pub subjects: &'static [&'static str],
    pub max_age_days: u64,
}

/// The system's 9 streams. Retention `Limits`, not `WorkQueue`: multiple
/// durable consumers read the same topics (Notification AND Analytics AND
/// Meilisearch reindexing consume `events.order.paid`). A WorkQueue
/// would deliver the message to only one of them.
pub const STREAMS: &[StreamSpec] = &[
    StreamSpec {
        name: "USERS",
        subjects: &["events.user.>", "events.address.>", "events.consent.>"],
        max_age_days: 90,
    },
    StreamSpec {
        name: "CATALOG",
        subjects: &[
            "events.product.>",
            "events.variant.>",
            "events.category.>",
            "events.brand.>",
            "events.player.>",
            "events.media.>",
        ],
        max_age_days: 30,
    },
    StreamSpec {
        name: "STOCK",
        subjects: &["events.inventory.>"],
        max_age_days: 90,
    },
    StreamSpec {
        name: "ORDERS",
        subjects: &["events.cart.>", "events.order.>"],
        max_age_days: 365,
    },
    StreamSpec {
        name: "PAYMENTS",
        subjects: &["events.payment.>", "events.refund.>"],
        max_age_days: 365,
    },
    StreamSpec {
        name: "NOTIFS",
        subjects: &["events.notification.>"],
        max_age_days: 30,
    },
    StreamSpec {
        name: "FULFILLMENT",
        subjects: &[
            "events.shipment.>",
            "events.return.>",
            "events.invoice.>",
            "events.credit_note.>",
        ],
        max_age_days: 365,
    },
    StreamSpec {
        name: "COMMERCE",
        subjects: &[
            "events.promotion.>",
            "events.giftcard.>",
            "events.review.>",
            "events.wishlist.>",
            "events.content.>",
        ],
        max_age_days: 90,
    },
    StreamSpec {
        name: "DLQ",
        subjects: &["dlq.>"],
        max_age_days: 30,
    },
];

/// Idempotent: creates any missing streams and updates existing ones.
/// Called when each service starts up—first come, first served.
pub async fn ensure_streams(js: &Context) -> Result<(), AppError> {
    for spec in STREAMS {
        js.get_or_create_stream(stream::Config {
            name: spec.name.to_string(),
            subjects: spec.subjects.iter().map(|s| s.to_string()).collect(),
            retention: stream::RetentionPolicy::Limits,
            storage: stream::StorageType::File,
            max_age: std::time::Duration::from_secs(spec.max_age_days * 86_400),
            // Server deduplication window based on Nats-Msg-Id: handles
            // reposts from the outbox relay after a crash.
            duplicate_window: std::time::Duration::from_secs(120),
            num_replicas: 1,
            ..Default::default()
        })
        .await
        .map_err(|e| AppError::Upstream(format!("cannot ensure stream {}: {e}", spec.name)))?;
        tracing::debug!(stream = spec.name, "jetstream stream ready");
    }
    Ok(())
}
