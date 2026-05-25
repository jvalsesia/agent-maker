use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub preamble: Option<String>,
    pub system_prompt: String,
    pub provider: String,
    pub model: String,
    pub has_override_key: bool,
    pub recent_n_override: Option<i16>,
    pub top_k_override: Option<i16>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentSummary {
    pub id: Uuid,
    pub name: String,
    pub preamble: Option<String>,
    pub provider: String,
    pub model: String,
    pub has_override_key: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub attached_skill_count: i64,
    pub conversation_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentUpsert {
    pub name: String,
    #[serde(default)]
    pub preamble: Option<String>,
    pub system_prompt: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub recent_n_override: Option<i16>,
    #[serde(default)]
    pub top_k_override: Option<i16>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CloneRequest {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutAgentKey {
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentResponse {
    pub agent: Agent,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SortField {
    #[default]
    Name,
    LastUsed,
    Created,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}


#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    #[serde(default)]
    pub sort: Option<SortField>,
    #[serde(default)]
    pub order: Option<SortOrder>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeySaved {
    pub has_override_key: bool,
    pub key_masked: String,
}
