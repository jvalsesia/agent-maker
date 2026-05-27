use super::catalog::{self, STARTER_AGENTS, STARTER_SKILLS};
use super::model::*;
use crate::agents::model::Agent;
use crate::error::{AppError, AppResult};
use crate::skills::model::Skill;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

const SUFFIX: &str = " (template)";

#[derive(Clone)]
pub struct TemplatesService {
    pub pool: PgPool,
}

impl TemplatesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Lists templates in the requested `locale`, substituting the localized
    /// `name`/`preamble`/`description` where a translation exists. When a slug
    /// has no variant for a non-`en` locale, the English text is returned with
    /// `is_fallback = true`. `en` (and any unsupported locale resolved upstream)
    /// always serves the canonical catalog with `is_fallback = false`.
    pub fn list(&self, category: Option<Category>, locale: &str) -> TemplatesList {
        let agents = STARTER_AGENTS
            .iter()
            .filter(|a| category.is_none_or(|c| a.category == c))
            .map(|a| {
                let variant = catalog::agent_variant(a.slug, locale);
                AgentTemplateSummary {
                    slug: a.slug,
                    name: variant.map_or(a.name, |v| v.name),
                    category: a.category.as_str(),
                    preamble: variant.map_or(a.preamble, |v| v.preamble),
                    suggested_skills: a.suggested_skills,
                    is_fallback: locale != "en" && variant.is_none(),
                }
            })
            .collect();
        let skills = STARTER_SKILLS
            .iter()
            .filter(|s| category.is_none_or(|c| s.category == c))
            .map(|s| {
                let variant = catalog::skill_variant(s.slug, locale);
                SkillTemplateSummary {
                    slug: s.slug,
                    name: variant.map_or(s.name, |v| v.name),
                    category: s.category.as_str(),
                    description: variant.map_or(s.description, |v| v.description),
                    is_fallback: locale != "en" && variant.is_none(),
                }
            })
            .collect();
        TemplatesList { agents, skills }
    }

    pub fn get_agent(&self, slug: &str) -> AppResult<AgentTemplateDetail> {
        let a = catalog::find_agent(slug)
            .ok_or_else(|| AppError::TemplateNotFound(format!("agent {slug}")))?;
        let suggested = a
            .suggested_skills
            .iter()
            .map(|s| {
                let sk = catalog::find_skill(s)
                    .expect("catalog::validate enforces every suggested slug resolves");
                SuggestedSkillPreview {
                    slug: sk.slug,
                    name: sk.name,
                    description: sk.description,
                }
            })
            .collect();
        Ok(AgentTemplateDetail {
            slug: a.slug,
            name: a.name,
            category: a.category.as_str(),
            preamble: a.preamble,
            system_prompt: a.system_prompt,
            default_provider: a.default_provider,
            default_model: a.default_model,
            suggested_skills: suggested,
        })
    }

    pub fn get_skill(&self, slug: &str) -> AppResult<SkillTemplateDetail> {
        let s = catalog::find_skill(slug)
            .ok_or_else(|| AppError::TemplateNotFound(format!("skill {slug}")))?;
        Ok(SkillTemplateDetail {
            slug: s.slug,
            name: s.name,
            category: s.category.as_str(),
            description: s.description,
            body: s.body,
        })
    }

    pub async fn adopt_agent(&self, slug: &str) -> AppResult<AdoptAgentResponse> {
        let template = catalog::find_agent(slug)
            .ok_or_else(|| AppError::TemplateNotFound(format!("agent {slug}")))?;

        let mut tx = self.pool.begin().await?;

        let agent_name = unique_name(&mut tx, "agents", template.name).await?;
        let agent: Agent = sqlx::query_as(
            r#"INSERT INTO agents
                 (name, preamble, system_prompt, provider, model)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, name, preamble, system_prompt, provider, model, has_override_key,
                         recent_n_override, top_k_override, response_language,
                         created_at, updated_at, last_used_at"#,
        )
        .bind(&agent_name)
        .bind(template.preamble)
        .bind(template.system_prompt)
        .bind(template.default_provider)
        .bind(template.default_model)
        .fetch_one(&mut *tx)
        .await?;

        let mut attached_skill_ids: Vec<Uuid> = Vec::new();
        for (position, sk_slug) in template.suggested_skills.iter().enumerate() {
            let sk_template = catalog::find_skill(sk_slug)
                .expect("catalog::validate enforces every suggested slug resolves");

            // Re-use an existing user skill if its name matches the template's
            // canonical name OR any " (template)" suffixed variant — this keeps
            // a second agent adoption from duplicating skills the user already has.
            let existing: Option<Uuid> = sqlx::query_scalar(
                r#"SELECT id FROM skills
                   WHERE name = $1 OR name LIKE $2
                   ORDER BY (name = $1) DESC, name ASC LIMIT 1"#,
            )
            .bind(sk_template.name)
            .bind(format!("{}{SUFFIX}%", sk_template.name))
            .fetch_optional(&mut *tx)
            .await?;

            let skill_id = if let Some(id) = existing {
                id
            } else {
                let final_name = unique_name(&mut tx, "skills", sk_template.name).await?;
                let inserted: Uuid = sqlx::query_scalar(
                    r#"INSERT INTO skills (name, description, body)
                       VALUES ($1, $2, $3) RETURNING id"#,
                )
                .bind(&final_name)
                .bind(sk_template.description)
                .bind(sk_template.body)
                .fetch_one(&mut *tx)
                .await?;
                inserted
            };

            sqlx::query(
                "INSERT INTO agent_skills (agent_id, skill_id, position) VALUES ($1, $2, $3)",
            )
            .bind(agent.id)
            .bind(skill_id)
            .bind(position as i16)
            .execute(&mut *tx)
            .await?;
            attached_skill_ids.push(skill_id);
        }

        tx.commit().await?;
        Ok(AdoptAgentResponse {
            agent,
            attached_skill_ids,
            warnings: Vec::new(),
        })
    }

    pub async fn adopt_skill(&self, slug: &str) -> AppResult<AdoptSkillResponse> {
        let template = catalog::find_skill(slug)
            .ok_or_else(|| AppError::TemplateNotFound(format!("skill {slug}")))?;

        let mut tx = self.pool.begin().await?;
        let final_name = unique_name(&mut tx, "skills", template.name).await?;
        let skill: Skill = sqlx::query_as(
            r#"INSERT INTO skills (name, description, body)
               VALUES ($1, $2, $3)
               RETURNING id, name, description, body, created_at, updated_at"#,
        )
        .bind(&final_name)
        .bind(template.description)
        .bind(template.body)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(AdoptSkillResponse {
            skill,
            warnings: Vec::new(),
        })
    }
}

/// Pick the first available name in the sequence
/// `<base>`, `<base> (template)`, `<base> (template) (2)`, `<base> (template) (3)`, …
async fn unique_name(
    tx: &mut Transaction<'_, Postgres>,
    table: &'static str,
    base: &str,
) -> AppResult<String> {
    // Pull every existing name that could collide in one query, then resolve
    // locally — avoids N round-trips in the suffix retry loop.
    let sql = format!(
        "SELECT name FROM {table} WHERE name = $1 OR name LIKE $2 OR name LIKE $3"
    );
    let rows: Vec<String> = sqlx::query_scalar(&sql)
        .bind(base)
        .bind(format!("{base}{SUFFIX}"))
        .bind(format!("{base}{SUFFIX} (%)"))
        .fetch_all(&mut **tx)
        .await?;

    if !rows.iter().any(|n| n == base) {
        return Ok(base.to_string());
    }
    let with_suffix = format!("{base}{SUFFIX}");
    if !rows.contains(&with_suffix) {
        return Ok(with_suffix);
    }
    for n in 2..1000 {
        let candidate = format!("{base}{SUFFIX} ({n})");
        if !rows.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(AppError::Internal(format!(
        "could not find unique adopt name for {base}"
    )))
}

// Service-level tests live in tests/templates.rs (Phase 2) — they need a
// live Postgres pool. Catalog-only assertions are in catalog::tests.
