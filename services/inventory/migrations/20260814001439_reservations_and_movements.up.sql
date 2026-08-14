-- Reservation at TTL: set at checkout, confirmed upon payment; otherwise, expired
-- by a periodic job that triggers the `events.inventory.reservation_expired` event.
CREATE TABLE stock_reservations (
    id           uuid PRIMARY KEY,
    stock_item_id uuid       NOT NULL REFERENCES stock_items (id) ON DELETE RESTRICT,
    variant_id   uuid        NOT NULL,
    quantity     integer     NOT NULL CHECK (quantity > 0),
    owner_type   text        NOT NULL CHECK (owner_type IN ('cart', 'order')),
    owner_id     uuid        NOT NULL,
    idempotency_key text,
    status       text        NOT NULL DEFAULT 'held'
                   CHECK (status IN ('held', 'confirmed', 'released', 'expired')),
    expires_at   timestamptz NOT NULL,
    confirmed_at timestamptz,
    released_at  timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);
-- The expiration job scans only active reservations.
CREATE INDEX stock_reservations_expiring_idx ON stock_reservations (expires_at)
    WHERE status = 'held';
CREATE INDEX stock_reservations_owner_idx    ON stock_reservations (owner_type, owner_id);
CREATE UNIQUE INDEX stock_reservations_idem_key ON stock_reservations (idempotency_key)
    WHERE idempotency_key IS NOT NULL;

-- Inventory ledger: append-only. Every change to stock_items has
-- exactly one entry here. This is the audit trail.
CREATE TABLE stock_movements (
    id             uuid PRIMARY KEY,
    stock_item_id  uuid        NOT NULL REFERENCES stock_items (id) ON DELETE RESTRICT,
    variant_id     uuid        NOT NULL,
    warehouse_id   uuid        NOT NULL,
    delta          integer     NOT NULL CHECK (delta <> 0),
    balance_after  integer     NOT NULL,
    reason         text        NOT NULL CHECK (reason IN
                     ('receipt', 'sale', 'return', 'adjustment', 'damage',
                      'reservation_confirmed', 'correction', 'initial')),
    reference_type text,
    reference_id   uuid,
    actor_user_id  uuid,
    note           text,
    created_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX stock_movements_item_idx      ON stock_movements (stock_item_id, created_at DESC);
CREATE INDEX stock_movements_reference_idx ON stock_movements (reference_type, reference_id);