-- Standardized options template (size / color / …) rather than a JSONB:
-- essential for generating Meilisearch facets and front-end selectors.
CREATE TABLE option_types (
    id         uuid PRIMARY KEY,
    code       text        NOT NULL,          -- 'size', 'color'
    position   integer     NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX option_types_code_key ON option_types (code);

CREATE TABLE option_type_translations (
    option_type_id uuid        NOT NULL REFERENCES option_types (id) ON DELETE CASCADE,
    locale         locale_code NOT NULL,
    name           text        NOT NULL,
    PRIMARY KEY (option_type_id, locale)
);

CREATE TABLE option_values (
    id             uuid PRIMARY KEY,
    option_type_id uuid        NOT NULL REFERENCES option_types (id) ON DELETE CASCADE,
    code           text        NOT NULL,      -- 'xl', 'navy'
    hex_color      char(7) CHECK (hex_color ~ '^#[0-9a-fA-F]{6}$'),
    position       integer     NOT NULL DEFAULT 0,
    created_at     timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX option_values_code_key ON option_values (option_type_id, code);

CREATE TABLE option_value_translations (
    option_value_id uuid        NOT NULL REFERENCES option_values (id) ON DELETE CASCADE,
    locale          locale_code NOT NULL,
    name            text        NOT NULL,
    PRIMARY KEY (option_value_id, locale)
);

CREATE TABLE product_variants (
    id               uuid PRIMARY KEY,
    product_id       uuid        NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    sku              text        NOT NULL,
    price            money_minor,                     -- NULL => inherits from products.base_price
    compare_at_price money_minor,
    weight_grams     integer CHECK (weight_grams >= 0),
    barcode          text,
    position         integer     NOT NULL DEFAULT 0,
    is_active        boolean     NOT NULL DEFAULT true,
    version          integer     NOT NULL DEFAULT 0,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    deleted_at       timestamptz
);
CREATE UNIQUE INDEX product_variants_sku_key     ON product_variants (sku) WHERE deleted_at IS NULL;
CREATE        INDEX product_variants_product_idx ON product_variants (product_id)
    WHERE deleted_at IS NULL;

CREATE TABLE variant_option_values (
    variant_id      uuid NOT NULL REFERENCES product_variants (id) ON DELETE CASCADE,
    option_value_id uuid NOT NULL REFERENCES option_values (id)   ON DELETE RESTRICT,
    PRIMARY KEY (variant_id, option_value_id)
);
CREATE INDEX variant_option_values_value_idx ON variant_option_values (option_value_id);

CREATE TRIGGER product_variants_set_updated_at BEFORE UPDATE ON product_variants
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();