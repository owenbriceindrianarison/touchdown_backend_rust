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
CREATE INDEX outbox_events_pending_idx ON outbox_events (available_at, id)
    WHERE published_at IS NULL;

-- Inbox: Catalog consumes events.media.* to refresh its cached URLs.
CREATE TABLE processed_events (
    event_id     uuid        NOT NULL,
    consumer     text        NOT NULL,
    subject      text        NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (event_id, consumer)
);
CREATE INDEX processed_events_ttl_idx ON processed_events (processed_at);

-- Meilisearch reindexing file: decouples the SQL update from the indexing process.
CREATE TABLE search_index_queue (
    product_id   uuid PRIMARY KEY REFERENCES products (id) ON DELETE CASCADE,
    operation    text        NOT NULL CHECK (operation IN ('upsert', 'delete')),
    attempts     smallint    NOT NULL DEFAULT 0,
    last_error   text,
    enqueued_at  timestamptz NOT NULL DEFAULT now(),
    processed_at timestamptz
);
CREATE INDEX search_index_queue_pending_idx ON search_index_queue (enqueued_at)
    WHERE processed_at IS NULL;