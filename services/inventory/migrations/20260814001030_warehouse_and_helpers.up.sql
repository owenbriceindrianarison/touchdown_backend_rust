CREATE TABLE warehouses (
    id         uuid PRIMARY KEY,
    code       text         NOT NULL,
    name       text         NOT NULL,
    country    country_code NOT NULL,
    priority   integer      NOT NULL DEFAULT 0,   -- multi-warehouse allocation order
    is_active  boolean      NOT NULL DEFAULT true,
    created_at timestamptz  NOT NULL DEFAULT now(),
    updated_at timestamptz  NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX warehouses_code_key ON warehouses (code);

CREATE TABLE stock_items (
    id            uuid PRIMARY KEY,
    variant_id    uuid        NOT NULL,          -- Catalog ref., no FK
    sku           text        NOT NULL,          -- snapshot, for exports/alerts
    warehouse_id  uuid        NOT NULL REFERENCES warehouses (id) ON DELETE RESTRICT,
    on_hand       integer     NOT NULL DEFAULT 0 CHECK (on_hand  >= 0),
    reserved      integer     NOT NULL DEFAULT 0 CHECK (reserved >= 0),
    safety_stock  integer     NOT NULL DEFAULT 0 CHECK (safety_stock >= 0),
    reorder_point integer     NOT NULL DEFAULT 0,
    backorderable boolean     NOT NULL DEFAULT false,
    version       integer     NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    -- Key principle: We never reserve more than the physical space.
    CONSTRAINT stock_items_reserved_lte_on_hand CHECK (reserved <= on_hand)
);
CREATE UNIQUE INDEX stock_items_variant_warehouse_key ON stock_items (variant_id, warehouse_id);
CREATE        INDEX stock_items_variant_idx           ON stock_items (variant_id);
-- Replenishment Alert Request: Only the affected lines.
CREATE        INDEX stock_items_low_idx               ON stock_items (warehouse_id)
    WHERE on_hand - reserved <= reorder_point;

CREATE TRIGGER warehouses_set_updated_at  BEFORE UPDATE ON warehouses
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER stock_items_set_updated_at BEFORE UPDATE ON stock_items
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

INSERT INTO warehouses (id, code, name, country, priority)
VALUES ('01930000-0000-7000-8000-000000000001', 'MAIN', 'Main Warehouse', 'EN', 0);