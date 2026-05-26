use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Writing,
    Research,
    Productivity,
    Coding,
    Learning,
    Wellbeing,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Writing => "writing",
            Category::Research => "research",
            Category::Productivity => "productivity",
            Category::Coding => "coding",
            Category::Learning => "learning",
            Category::Wellbeing => "wellbeing",
        }
    }

    pub fn parse(s: &str) -> Option<Category> {
        Some(match s.to_ascii_lowercase().as_str() {
            "writing" => Category::Writing,
            "research" => Category::Research,
            "productivity" => Category::Productivity,
            "coding" => Category::Coding,
            "learning" => Category::Learning,
            "wellbeing" => Category::Wellbeing,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SkillTemplate {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub body: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct AgentTemplate {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub preamble: &'static str,
    pub system_prompt: &'static str,
    pub default_provider: &'static str,
    pub default_model: &'static str,
    pub suggested_skills: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTemplateSummary {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub preamble: &'static str,
    pub suggested_skills: &'static [&'static str],
    /// `true` when the `en` variant was returned because no translation exists
    /// for the requested locale. See spec Section 5.
    pub is_fallback: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillTemplateSummary {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    /// `true` when the `en` variant was returned because no translation exists
    /// for the requested locale. See spec Section 5.
    pub is_fallback: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplatesList {
    pub agents: Vec<AgentTemplateSummary>,
    pub skills: Vec<SkillTemplateSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedSkillPreview {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentTemplateDetail {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub preamble: &'static str,
    pub system_prompt: &'static str,
    pub default_provider: &'static str,
    pub default_model: &'static str,
    pub suggested_skills: Vec<SuggestedSkillPreview>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillTemplateDetail {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub body: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub field: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdoptAgentResponse {
    pub agent: crate::agents::model::Agent,
    pub attached_skill_ids: Vec<Uuid>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdoptSkillResponse {
    pub skill: crate::skills::model::Skill,
    pub warnings: Vec<Warning>,
}
