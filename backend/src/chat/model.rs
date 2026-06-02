use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request body for `POST /api/conversations/:id/chat`. `content` is required
/// unless `retry` is true, in which case the existing trailing user message is
/// reused. `recent_n` / `top_k` override the agent/global memory parameters.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ChatStartRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub retry: bool,
    #[serde(default)]
    pub recent_n: Option<i16>,
    #[serde(default)]
    pub top_k: Option<i16>,
    /// Active UI locale (F09). Used to pick the response language when the
    /// agent's `response_language` is `auto`.
    #[serde(default)]
    pub locale: Option<String>,
}

/// A recalled earlier turn as surfaced in the `meta` SSE frame.
#[derive(Debug, Clone, Serialize)]
pub struct RecalledTurn {
    pub message_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub similarity: f32,
}

/// An inline notice about `@mention` handling, surfaced on the `meta` frame for
/// the client to localize. `code` is `"overflow"` (matched aliases ignored past
/// the per-turn cap) or `"unknown"` (handles matching no attached sub-agent).
#[derive(Debug, Clone, Serialize)]
pub struct SubagentNotice {
    pub code: String,
    pub aliases: Vec<String>,
}

/// One frame of the chat SSE stream. Serialized as `{ "type": "...", ... }`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Meta {
        user_message_id: Uuid,
        assistant_message_id: Uuid,
        model: String,
        degraded: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        degraded_reason: Option<String>,
        recalled: Vec<RecalledTurn>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        notices: Vec<SubagentNotice>,
    },
    /// A completed delegated (F11) sub-agent turn, emitted once per honored
    /// mention before the parent's `chunk` frames.
    Subagent {
        message_id: Uuid,
        alias: String,
        agent_id: Uuid,
        agent_name: String,
        content: String,
        /// `"complete"` or `"error"`.
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    Chunk {
        content: String,
    },
    Done {
        status: String,
        token_count: i32,
        #[serde(skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
    },
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        provider: Option<String>,
    },
}
