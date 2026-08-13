CREATE TABLE product_media (
    product_id uuid        NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    media_id   uuid        NOT NULL,               -- réf. Media service
    variant_id uuid        REFERENCES product_variants (id) ON DELETE CASCADE,
    role       text        NOT NULL DEFAULT 'gallery'
                 CHECK (role IN ('main', 'gallery', 'thumbnail', 'size_chart')),
    url        text        NOT NULL,               -- denormalized cache (SSR without round-trip)
    thumb_url  text,
    position   integer     NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (product_id, media_id)
);
CREATE INDEX product_media_variant_idx ON product_media (variant_id);
-- Only one main image per product.
CREATE UNIQUE INDEX product_media_one_main ON product_media (product_id)
    WHERE role = 'main' AND variant_id IS NULL;

-- Signature products / products worn by a player.
CREATE TABLE product_players (
    product_id uuid        NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    player_id  uuid        NOT NULL REFERENCES players (id)  ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (product_id, player_id)
);
CREATE INDEX product_players_player_idx ON product_players (player_id);