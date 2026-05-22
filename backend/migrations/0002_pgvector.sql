CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE message_embeddings (
    message_id      UUID PRIMARY KEY REFERENCES messages (id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    model           TEXT NOT NULL,
    dim             INTEGER NOT NULL,
    embedding       vector(1536) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_message_embeddings_conv ON message_embeddings (conversation_id);
CREATE INDEX idx_message_embeddings_ann
    ON message_embeddings USING hnsw (embedding vector_cosine_ops);
