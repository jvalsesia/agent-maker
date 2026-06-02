-- F11 sub-agents: agent → agent delegation links + delegated-turn marking.

CREATE TABLE agent_subagents (
    parent_id   UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    child_id    UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    alias       TEXT NOT NULL,
    description TEXT,
    position    SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 9),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (parent_id, child_id),
    -- Deferred so a full reorder can assign final positions within one transaction
    -- without tripping the uniqueness check on an intermediate state.
    CONSTRAINT uq_agent_subagents_pos UNIQUE (parent_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT uq_agent_subagents_alias UNIQUE (parent_id, alias),
    CHECK (parent_id <> child_id)
);
CREATE INDEX idx_agent_subagents_child ON agent_subagents (child_id);

ALTER TABLE messages
    ADD COLUMN subagent_alias    TEXT,
    ADD COLUMN subagent_agent_id UUID REFERENCES agents (id) ON DELETE SET NULL;
