-- F03: maintain skills.updated_at automatically on UPDATE.
-- Reuses the generic touch_updated_at() function defined in 0003_agents_touch.sql.

CREATE TRIGGER skills_touch
BEFORE UPDATE ON skills
FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
