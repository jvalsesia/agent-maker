use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub message_count: i32,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub status: String,
    pub model: Option<String>,
    pub token_count: Option<i32>,
    pub finish_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    /// Earlier turns recalled by F08 for this (assistant) message. Empty for
    /// user messages and assistant turns composed without retrieval. Not a
    /// column — populated by `list_messages`.
    #[sqlx(skip)]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recalled: Vec<RecalledRef>,
}

/// A reference to an earlier message recalled into an assistant turn's prompt,
/// shown under the "Recalled N earlier turns" indicator.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct RecalledRef {
    pub message_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub similarity: f32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CreateInput {
    #[serde(default)]
    pub title: Option<String>,
}

/// Fields for inserting a new message via `ConversationsService::insert_message`.
#[derive(Debug, Clone)]
pub struct NewMessage<'a> {
    pub role: &'a str,
    pub content: &'a str,
    pub status: &'a str,
    pub model: Option<&'a str>,
    pub token_count: Option<i32>,
    pub finish_reason: Option<&'a str>,
}

impl<'a> NewMessage<'a> {
    pub fn user(content: &'a str) -> Self {
        Self { role: "user", content, status: "complete", model: None, token_count: None, finish_reason: None }
    }
    pub fn assistant(content: &'a str, status: &'a str, model: Option<&'a str>) -> Self {
        Self { role: "assistant", content, status, model, token_count: None, finish_reason: None }
    }
}

/// Max length of an auto-generated conversation title (F06 rule, applied on the
/// first user message sent through the chat runtime).
pub const AUTO_TITLE_MAX: usize = 60;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameInput {
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationsList {
    pub conversations: Vec<Conversation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversationResponse {
    pub conversation: Conversation,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ListMessagesResponse {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

pub const DEFAULT_TITLE: &str = "New conversation";
pub const TITLE_MIN: usize = 1;
pub const TITLE_MAX: usize = 80;
