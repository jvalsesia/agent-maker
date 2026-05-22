use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AttachedSkill {
    pub skill_id: Uuid,
    pub name: String,
    pub description: String,
    pub position: i16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachReplaceInput {
    pub skill_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentsResponse {
    pub attached: Vec<AttachedSkill>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComposePreview {
    pub composed: String,
    pub length_chars: i64,
    pub model_context_chars: i64,
    pub fraction: f32,
    pub warning: Option<String>,
}
