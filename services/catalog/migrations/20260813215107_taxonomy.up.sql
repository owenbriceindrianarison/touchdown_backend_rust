CREATE TABLE categories (
    id            uuid PRIMARY KEY,
    parent_id     uuid REFERENCES categories (id) ON DELETE RESTRICT,
    code          text        NOT NULL,          -- 'helmet', 'jersey' (stable, untranslated)
    icon_media_id uuid,                          -- réf. Media, no FK
    icon_url      text,                          -- denormalized cache for SSR
    position      integer     NOT NULL DEFAULT 0,
    is_active     boolean     NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    deleted_at    timestamptz
);
CREATE UNIQUE INDEX categories_code_key   ON categories (code) WHERE deleted_at IS NULL;
CREATE        INDEX categories_parent_idx ON categories (parent_id);

CREATE TABLE category_translations (
    category_id     uuid        NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    locale          locale_code NOT NULL,
    name            text        NOT NULL,
    slug            text        NOT NULL,
    description     text,
    seo_title       text,
    seo_description text,
    PRIMARY KEY (category_id, locale)
);
CREATE UNIQUE INDEX category_translations_slug_key ON category_translations (locale, slug);

CREATE TABLE brands (
    id            uuid PRIMARY KEY,
    code          text        NOT NULL,
    logo_media_id uuid,
    logo_url      text,
    website_url   text,
    position      integer     NOT NULL DEFAULT 0,
    is_active     boolean     NOT NULL DEFAULT true,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    deleted_at    timestamptz
);
CREATE UNIQUE INDEX brands_code_key ON brands (code) WHERE deleted_at IS NULL;

CREATE TABLE brand_translations (
    brand_id    uuid        NOT NULL REFERENCES brands (id) ON DELETE CASCADE,
    locale      locale_code NOT NULL,
    name        text        NOT NULL,
    slug        text        NOT NULL,
    description text,
    PRIMARY KEY (brand_id, locale)
);
CREATE UNIQUE INDEX brand_translations_slug_key ON brand_translations (locale, slug);

CREATE TABLE players (
    id               uuid PRIMARY KEY,
    code             text        NOT NULL,
    first_name       text        NOT NULL,
    last_name        text        NOT NULL,
    jersey_number    smallint,
    field_position   text,                       -- 'QB', 'WR', 'TE'…
    team             text,
    portrait_media_id uuid,
    portrait_url     text,
    position         integer     NOT NULL DEFAULT 0,
    is_active        boolean     NOT NULL DEFAULT true,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),
    deleted_at       timestamptz
);
CREATE UNIQUE INDEX players_code_key ON players (code) WHERE deleted_at IS NULL;

CREATE TABLE player_translations (
    player_id uuid        NOT NULL REFERENCES players (id) ON DELETE CASCADE,
    locale    locale_code NOT NULL,
    slug      text        NOT NULL,
    bio       text,
    PRIMARY KEY (player_id, locale)
);
CREATE UNIQUE INDEX player_translations_slug_key ON player_translations (locale, slug);

CREATE TRIGGER categories_set_updated_at BEFORE UPDATE ON categories
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER brands_set_updated_at     BEFORE UPDATE ON brands
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER players_set_updated_at    BEFORE UPDATE ON players
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();