CREATE TABLE products (
    id                 uuid PRIMARY KEY,
    sku_root           text          NOT NULL,
    category_id        uuid          NOT NULL REFERENCES categories (id) ON DELETE RESTRICT,
    brand_id           uuid          REFERENCES brands (id) ON DELETE SET NULL,
    status             text          NOT NULL DEFAULT 'draft'
                         CHECK (status IN ('draft', 'active', 'archived')),
    base_price         money_minor   NOT NULL,          -- centimes
    compare_at_price   money_minor,
    currency           currency_code NOT NULL DEFAULT 'EUR',
    tax_category_code  text          NOT NULL DEFAULT 'standard',   -- Resolved by the Tax department
    -- Restored the Mongo `color: Float[]`: accent color for the product page.
    accent_r           real          CHECK (accent_r BETWEEN 0 AND 1),
    accent_g           real          CHECK (accent_g BETWEEN 0 AND 1),
    accent_b           real          CHECK (accent_b BETWEEN 0 AND 1),
    weight_grams       integer       CHECK (weight_grams >= 0),
    is_featured        boolean       NOT NULL DEFAULT false,
    published_at       timestamptz,
    version            integer       NOT NULL DEFAULT 0,
    created_at         timestamptz   NOT NULL DEFAULT now(),
    updated_at         timestamptz   NOT NULL DEFAULT now(),
    deleted_at         timestamptz
);

CREATE UNIQUE INDEX products_sku_root_key  ON products (sku_root) WHERE deleted_at IS NULL;
CREATE        INDEX products_category_idx  ON products (category_id)
    WHERE status = 'active' AND deleted_at IS NULL;
CREATE        INDEX products_brand_idx     ON products (brand_id)
    WHERE status = 'active' AND deleted_at IS NULL;
-- Default sorting for the storefront (new items): index specifying the listing order.
CREATE        INDEX products_listing_idx   ON products (published_at DESC, id)
    WHERE status = 'active' AND deleted_at IS NULL;
-- CREATE        INDEX products_featured_idx  ON products (position_placeholder_removed)
--     WHERE false;   -- placeholder removed, see 0003b

CREATE TABLE product_translations (
    product_id        uuid        NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    locale            locale_code NOT NULL,
    name              text        NOT NULL,
    slug              text        NOT NULL,
    short_description text,
    description       text,
    seo_title         text,
    seo_description   text,
    search_keywords   text,
    PRIMARY KEY (product_id, locale)
);
CREATE UNIQUE INDEX product_translations_slug_key ON product_translations (locale, slug);
-- Search fallback if Meilisearch is unavailable (degraded, not operating normally).
CREATE INDEX product_translations_name_trgm ON product_translations
    USING gin (name gin_trgm_ops);

CREATE TRIGGER products_set_updated_at BEFORE UPDATE ON products
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();