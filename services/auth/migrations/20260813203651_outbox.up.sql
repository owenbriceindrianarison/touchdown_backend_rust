-- Transactional outbox. The handler writes the business data AND the event within the
-- SAME transaction; an asynchronous relay then publishes to JetStream and
-- sets the `published_at` field. Guarantees “at least once” without 2PC.
CREATE TABLE outbox_events (
    id            uuid PRIMARY KEY,          -- becomes the Nats-Msg-Id
    subject       text        NOT NULL,      -- ex: 'events.user.registered'
    payload       jsonb       NOT NULL,
    headers       jsonb       NOT NULL DEFAULT '{}'::jsonb,
    trace_id      text,
    attempts      smallint    NOT NULL DEFAULT 0,
    last_error    text,
    available_at  timestamptz NOT NULL DEFAULT now(),
    published_at  timestamptz,
    created_at    timestamptz NOT NULL DEFAULT now()
);

-- Relay work index: pending messages only
CREATE INDEX outbox_events_pending_idx ON outbox_events (available_at, id)
    WHERE published_at IS NULL;