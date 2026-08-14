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