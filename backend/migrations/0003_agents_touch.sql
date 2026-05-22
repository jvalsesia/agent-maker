-- F02: maintain agents.updated_at automatically on UPDATE.
-- The function is intentionally generic so F03 (skills) and F06 (conversations)
-- can attach the same trigger to their tables without redefining it.

CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER agents_touch
BEFORE UPDATE ON agents
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
