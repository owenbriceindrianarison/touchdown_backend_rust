CREATE EXTENSION IF NOT EXISTS citext;
CREATE EXTENSION IF NOT EXISTS pg_trgm;   -- Search fallback if Meilisearch is down
CREATE EXTENSION IF NOT EXISTS unaccent;

CREATE DOMAIN locale_code   AS TEXT    CHECK (VALUE IN ('fr', 'en'));
CREATE DOMAIN currency_code AS CHAR(3) CHECK (VALUE ~ '^[A-Z]{3}$');
CREATE DOMAIN money_minor   AS BIGINT  CHECK (VALUE >= 0);

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;