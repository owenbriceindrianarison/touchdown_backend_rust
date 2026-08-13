CREATE TABLE users (
    id                     uuid PRIMARY KEY,
    email                  citext      NOT NULL,
    email_verified_at      timestamptz,
    password_hash          text        NOT NULL,
    first_name             text,
    last_name              text,
    phone                  text,
    locale                 locale_code NOT NULL DEFAULT 'en',
    role                   text        NOT NULL DEFAULT 'customer'
                             CHECK (role IN ('customer', 'staff', 'admin')),
    status                 text        NOT NULL DEFAULT 'active'
                             CHECK (status IN ('active', 'locked', 'disabled', 'anonymized')),
    failed_login_attempts  smallint    NOT NULL DEFAULT 0,
    locked_until           timestamptz,
    last_login_at          timestamptz,
    tos_accepted_at        timestamptz,
    version                integer     NOT NULL DEFAULT 0,
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    deleted_at             timestamptz
);

-- Email uniqueness applies only to active accounts:
-- A deleted account should not prevent re-registration.
CREATE UNIQUE INDEX users_email_key      ON users (email) WHERE deleted_at IS NULL;
CREATE        INDEX users_role_idx       ON users (role)  WHERE deleted_at IS NULL;
CREATE        INDEX users_created_at_idx ON users (created_at DESC);

CREATE TRIGGER users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();