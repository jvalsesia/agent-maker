use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A joined view of one attached sub-agent — the shape returned by the list and
/// attach/update endpoints (an item under `attached`).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AttachedSubagent {
    pub child_id: Uuid,
    pub name: String,
    pub alias: String,
    pub description: Option<String>,
    pub position: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachInput {
    pub child_id: Uuid,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateInput {
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReorderInput {
    pub ordered_child_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubagentsResponse {
    pub attached: Vec<AttachedSubagent>,
}

/// Maximum sub-agents attachable to one parent (positions `0..=9`).
pub const MAX_SUBAGENTS: usize = 10;
/// Maximum alias length, matching `^[a-z0-9-]{1,30}$`.
pub const ALIAS_MAX: usize = 30;
/// Maximum "when to use" description length (mirrors the F03 skill description).
pub const DESCRIPTION_MAX: usize = 200;
