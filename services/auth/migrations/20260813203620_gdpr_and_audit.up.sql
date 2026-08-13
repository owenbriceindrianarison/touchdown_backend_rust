CREATE TABLE user_consents (
    id         uuid PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    purpose    text        NOT NULL
                 CHECK (purpose IN ('marketing_email', 'analytics', 'personalization')),
    granted    boolean     NOT NULL,
    source     text        NOT NULL DEFAULT 'account'
                 CHECK (source IN ('signup', 'account', 'banner', 'admin')),
    ip         inet,
    created_at timestamptz NOT NULL DEFAULT now()
);
-- Append-only: The consent history serves as GDPR evidence.
CREATE INDEX user_consents_user_purpose_idx ON user_consents (user_id, purpose, created_at DESC);

CREATE TABLE data_export_requests (
    id           uuid PRIMARY KEY,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status       text        NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending', 'processing', 'ready', 'failed', 'expired')),
    media_id     uuid,                       -- archive uploaded to Media (signed URL)
    error        text,
    requested_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    expires_at   timestamptz
);
CREATE INDEX data_export_requests_user_idx ON data_export_requests (user_id, requested_at DESC);

CREATE TABLE erasure_requests (
    id           uuid PRIMARY KEY,
    user_id      uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status       text        NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending', 'processing', 'completed', 'rejected')),
    reason       text,
    requested_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);
CREATE INDEX erasure_requests_status_idx ON erasure_requests (status)
    WHERE status IN ('pending', 'processing');

CREATE TABLE audit_log (
    id             uuid PRIMARY KEY,
    actor_user_id  uuid,
    actor_role     text,
    action         text        NOT NULL,     -- ex: 'user.role_changed'
    target_type    text,
    target_id      uuid,
    ip             inet,
    user_agent     text,
    metadata       jsonb       NOT NULL DEFAULT '{}'::jsonb,
    created_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_log_actor_idx  ON audit_log (actor_user_id, created_at DESC);
CREATE INDEX audit_log_target_idx ON audit_log (target_type, target_id, created_at DESC);