use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SkillSummary {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attached_agent_count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UsingAgent {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillDetail {
    pub skill: Skill,
    pub using_agents: Vec<UsingAgent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillUpsert {
    pub name: String,
    pub description: String,
    pub body: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct CloneRequest {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillResponse {
    pub skill: Skill,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SortField {
    #[default]
    Name,
    Attached,
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
