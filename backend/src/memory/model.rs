use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct MemoryTurn {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievedTurn {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryBlock {
    pub conversation_id: Uuid,
    pub agent_id: Uuid,
    pub effective_n: i16,
    pub effective_k: i16,
    pub recent: Vec<MemoryTurn>,
    pub retrieved: Vec<RetrievedTurn>,
    pub degraded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub degraded_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedReport {
    pub embedded: usize,
    pub skipped_already_present: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClearReport {
    pub removed: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryQuery {
    pub query: String,
    #[serde(default)]
    pub recent_n: Option<i16>,
    #[serde(default)]
    pub top_k: Option<i16>,
}

pub const RECENT_N_MIN: i16 = 4;
pub const RECENT_N_MAX: i16 = 30;
pub const TOP_K_MIN: i16 = 0;
pub const TOP_K_MAX: i16 = 10;
pub const QUERY_MAX_CHARS: usize = 4_000;
pub const EMBEDDING_MODEL: &str = "text-embedding-3-small";
pub const EMBEDDING_DIM: i32 = 1536;
