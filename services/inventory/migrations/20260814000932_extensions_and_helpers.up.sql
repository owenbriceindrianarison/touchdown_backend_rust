CREATE DOMAIN country_code AS CHAR(2) CHECK (VALUE ~ '^[A-Z]{2}$');

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$;