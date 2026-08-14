CREATE DOMAIN locale_code AS TEXT CHECK (VALUE IN ('fr', 'en'));

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;