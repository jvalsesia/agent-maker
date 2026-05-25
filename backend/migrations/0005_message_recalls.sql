-- F07 Chat Runtime: per-assistant-message recall attribution + finish reason.

ALTER TABLE messages ADD COLUMN finish_reason TEXT;

CREATE TABLE message_recalls (
    assistant_message_id UUID NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    recalled_message_id  UUID NOT NULL REFERENCES messages (id) ON DELETE CASCADE,
    similarity           REAL NOT NULL,
    position             INTEGER NOT NULL,
    PRIMARY KEY (assistant_message_id, recalled_message_id)
);

CREATE INDEX idx_message_recalls_assistant ON message_recalls (assistant_message_id);
