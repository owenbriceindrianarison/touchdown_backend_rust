CREATE TABLE media_assets (
    id                uuid PRIMARY KEY,
    bucket            text        NOT NULL,
    object_key        text        NOT NULL,
    original_filename text        NOT NULL,
    mime_type         text        NOT NULL,
    size_bytes        bigint      NOT NULL CHECK (size_bytes > 0),
    width             integer,
    height            integer,
    checksum_sha256   bytea,
    status            text        NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'processing', 'ready', 'failed')),
    visibility        text        NOT NULL DEFAULT 'public'
                        CHECK (visibility IN ('public', 'private')),
    owner_type        text        CHECK (owner_type IN
                        ('product', 'variant', 'category', 'brand', 'player',
                         'content', 'invoice', 'gdpr_export', 'review')),
    owner_id          uuid,
    uploaded_by       uuid,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    deleted_at        timestamptz
);
CREATE UNIQUE INDEX media_assets_object_key ON media_assets (bucket, object_key);
CREATE        INDEX media_assets_owner_idx  ON media_assets (owner_type, owner_id)
    WHERE deleted_at IS NULL;
-- Upload deduplication: same binary content = same reused asset.
CREATE        INDEX media_assets_checksum_idx ON media_assets (checksum_sha256)
    WHERE deleted_at IS NULL;

-- Derived renderings (thumbnails, modern formats).
CREATE TABLE media_renditions (
    id          uuid PRIMARY KEY,
    media_id    uuid        NOT NULL REFERENCES media_assets (id) ON DELETE CASCADE,
    kind        text        NOT NULL CHECK (kind IN
                  ('thumb', 'small', 'medium', 'large', 'webp', 'avif')),
    object_key  text        NOT NULL,
    mime_type   text        NOT NULL,
    width       integer     NOT NULL,
    height      integer     NOT NULL,
    size_bytes  bigint      NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX media_renditions_kind_key ON media_renditions (media_id, kind);

CREATE TABLE media_translations (
    media_id uuid        NOT NULL REFERENCES media_assets (id) ON DELETE CASCADE,
    locale   locale_code NOT NULL,
    alt_text text,
    caption  text,
    PRIMARY KEY (media_id, locale)
);

-- Direct browser upload -> MinIO/R2 via a pre-signed URL. The session tracks
-- the intent so that uploads that are never confirmed can be cleaned up.
CREATE TABLE upload_sessions (
    id           uuid PRIMARY KEY,
    media_id     uuid        NOT NULL REFERENCES media_assets (id) ON DELETE CASCADE,
    upload_url   text        NOT NULL,
    requested_by uuid        NOT NULL,
    expires_at   timestamptz NOT NULL,
    completed_at timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX upload_sessions_stale_idx ON upload_sessions (expires_at)
    WHERE completed_at IS NULL;

CREATE TRIGGER media_assets_set_updated_at BEFORE UPDATE ON media_assets
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();