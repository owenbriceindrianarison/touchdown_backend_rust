-- Refresh token rotation by “family”: with each refresh, the old token is
-- revoked and a new one is issued with the same family_id. If a token that has already
-- been revoked is reused => reuse detected => the ENTIRE family is revoked (token theft).
CREATE TABLE refresh_tokens (
    id             uuid PRIMARY KEY,
    user_id        uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    family_id      uuid        NOT NULL,
    parent_id      uuid        REFERENCES refresh_tokens (id) ON DELETE SET NULL,
    token_hash     bytea       NOT NULL,          -- SHA-256 du token, jamais le token
    expires_at     timestamptz NOT NULL,
    revoked_at     timestamptz,
    revoked_reason text CHECK (revoked_reason IN
                     ('rotated', 'logout', 'reuse_detected', 'password_changed', 'admin')),
    user_agent     text,
    ip             inet,
    created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX refresh_tokens_hash_key   ON refresh_tokens (token_hash);
CREATE        INDEX refresh_tokens_user_idx   ON refresh_tokens (user_id);
CREATE        INDEX refresh_tokens_family_idx ON refresh_tokens (family_id);
CREATE        INDEX refresh_tokens_active_idx ON refresh_tokens (expires_at)
    WHERE revoked_at IS NULL;

CREATE TABLE password_reset_tokens (
    id         uuid PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash bytea       NOT NULL,
    expires_at timestamptz NOT NULL,
    used_at    timestamptz,
    ip         inet,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX password_reset_tokens_hash_key ON password_reset_tokens (token_hash);
CREATE        INDEX password_reset_tokens_user_idx ON password_reset_tokens (user_id)
    WHERE used_at IS NULL;

CREATE TABLE email_verification_tokens (
    id         uuid PRIMARY KEY,
    user_id    uuid        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    email      citext      NOT NULL,
    token_hash bytea       NOT NULL,
    expires_at timestamptz NOT NULL,
    used_at    timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX email_verification_tokens_hash_key ON email_verification_tokens (token_hash);
CREATE        INDEX email_verification_tokens_user_idx ON email_verification_tokens (user_id);