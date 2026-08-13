CREATE TABLE addresses (
    id                  uuid PRIMARY KEY,
    user_id             uuid         NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    label               text,
    kind                text         NOT NULL DEFAULT 'both'
                          CHECK (kind IN ('shipping', 'billing', 'both')),
    first_name          text         NOT NULL,
    last_name           text         NOT NULL,
    company             text,
    line1               text         NOT NULL,
    line2               text,
    postal_code         text         NOT NULL,
    city                text         NOT NULL,
    state               text,
    country             country_code NOT NULL,
    phone               text,
    is_default_shipping boolean      NOT NULL DEFAULT false,
    is_default_billing  boolean      NOT NULL DEFAULT false,
    created_at          timestamptz  NOT NULL DEFAULT now(),
    updated_at          timestamptz  NOT NULL DEFAULT now(),
    deleted_at          timestamptz
);

CREATE INDEX addresses_user_idx ON addresses (user_id) WHERE deleted_at IS NULL;

-- Only one default address of each type per user.
CREATE UNIQUE INDEX addresses_one_default_shipping
    ON addresses (user_id) WHERE is_default_shipping AND deleted_at IS NULL;
CREATE UNIQUE INDEX addresses_one_default_billing
    ON addresses (user_id) WHERE is_default_billing  AND deleted_at IS NULL;

CREATE TRIGGER addresses_set_updated_at
    BEFORE UPDATE ON addresses
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();