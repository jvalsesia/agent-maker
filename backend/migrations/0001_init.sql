-- F01 initial schema. pgvector extension is enabled in 0002.

CREATE TABLE settings (
    id              TEXT PRIMARY KEY DEFAULT 'singleton' CHECK (id = 'singleton'),
    default_provider        TEXT        NOT NULL DEFAULT 'anthropic',
    default_model_anthropic TEXT,
    default_model_openai    TEXT,
    default_model_openai_compat TEXT,
    recent_n        SMALLINT    NOT NULL DEFAULT 10 CHECK (recent_n BETWEEN 4 AND 30),
    top_k           SMALLINT    NOT NULL DEFAULT 5  CHECK (top_k     BETWEEN 0 AND 10),
    theme           TEXT        NOT NULL DEFAULT 'system' CHECK (theme IN ('light','dark','system')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO settings (id) VALUES ('singleton');

CREATE TABLE provider_keys (
    name        TEXT PRIMARY KEY CHECK (name IN ('anthropic','openai','openai_compat')),
    key_masked  TEXT NOT NULL,
    base_url    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE agents (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL UNIQUE,
    preamble     TEXT,
    system_prompt TEXT NOT NULL,
    provider     TEXT NOT NULL CHECK (provider IN ('anthropic','openai','openai_compat')),
    model        TEXT NOT NULL,
    has_override_key BOOLEAN NOT NULL DEFAULT FALSE,
    recent_n_override SMALLINT CHECK (recent_n_override BETWEEN 4 AND 30),
    top_k_override    SMALLINT CHECK (top_k_override BETWEEN 0 AND 10),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX idx_agents_last_used_at ON agents (last_used_at DESC NULLS LAST);

CREATE TABLE skills (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL UNIQUE,
    description  TEXT NOT NULL,
    body         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE agent_skills (
    agent_id   UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    skill_id   UUID NOT NULL REFERENCES skills (id) ON DELETE CASCADE,
    position   SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 19),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, skill_id),
    UNIQUE (agent_id, position)
);
CREATE INDEX idx_agent_skills_skill ON agent_skills (skill_id);

CREATE TABLE conversations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id        UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    message_count   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_conversations_agent_activity ON conversations (agent_id, last_activity_at DESC);

CREATE TABLE messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'complete' CHECK (status IN ('complete','stopped','error')),
    model           TEXT,
    token_count     INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_messages_conversation_created ON messages (conversation_id, created_at);
