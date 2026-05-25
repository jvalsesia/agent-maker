-- F09 internationalization. Adds the active UI locale to the settings
-- singleton and a per-agent response language override.

ALTER TABLE settings
    ADD COLUMN locale TEXT NOT NULL DEFAULT 'en'
    CHECK (locale IN ('en','pt-BR'));

ALTER TABLE agents
    ADD COLUMN response_language TEXT NOT NULL DEFAULT 'auto'
    CHECK (response_language IN ('auto','en','pt-BR'));
