use super::model::*;
use crate::error::{AppError, AppResult};
use crate::llm::ProviderName;
use sqlx::PgPool;
use std::collections::HashSet;
use std::str::FromStr;
use uuid::Uuid;

const MAX_ATTACHED: usize = 20;
const WARN_FRACTION: f32 = 0.80;

#[derive(Clone)]
pub struct AttachmentsService {
    pub pool: PgPool,
}

impl AttachmentsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, agent_id: Uuid) -> AppResult<Vec<AttachedSkill>> {
        self.ensure_agent_exists(agent_id).await?;
        let rows: Vec<AttachedSkill> = sqlx::query_as(
            r#"SELECT s.id AS skill_id, s.name, s.description, a.position
               FROM agent_skills a
               JOIN skills s ON s.id = a.skill_id
               WHERE a.agent_id = $1
               ORDER BY a.position ASC"#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn replace(
        &self,
        agent_id: Uuid,
        input: AttachReplaceInput,
    ) -> AppResult<AttachmentsResponse> {
        Self::validate(&input)?;
        self.ensure_agent_exists(agent_id).await?;
        self.ensure_skills_exist(&input.skill_ids).await?;

        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM agent_skills WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;

        for (idx, skill_id) in input.skill_ids.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO agent_skills (agent_id, skill_id, position)
                   VALUES ($1, $2, $3)"#,
            )
            .bind(agent_id)
            .bind(skill_id)
            .bind(idx as i16)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        let attached = self.list(agent_id).await?;
        let warnings = self.compose_warnings(agent_id).await?;
        Ok(AttachmentsResponse { attached, warnings })
    }

    pub async fn detach_one(&self, agent_id: Uuid, skill_id: Uuid) -> AppResult<()> {
        self.ensure_agent_exists(agent_id).await?;

        let mut tx = self.pool.begin().await?;
        let removed = sqlx::query("DELETE FROM agent_skills WHERE agent_id = $1 AND skill_id = $2")
            .bind(agent_id)
            .bind(skill_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if removed == 0 {
            return Err(AppError::NotFound(format!(
                "skill {skill_id} not attached to agent {agent_id}"
            )));
        }

        // Renumber remaining rows to close the gap. Iterating in ascending position
        // order means every new position is <= the old one, so we never collide with
        // an existing row (UNIQUE(agent_id, position)) and never exceed the CHECK
        // bound (position BETWEEN 0 AND 19).
        let remaining: Vec<(Uuid, i16)> = sqlx::query_as(
            r#"SELECT skill_id, position FROM agent_skills
               WHERE agent_id = $1 ORDER BY position ASC"#,
        )
        .bind(agent_id)
        .fetch_all(&mut *tx)
        .await?;

        for (idx, (sid, old_pos)) in remaining.iter().enumerate() {
            let new_pos = idx as i16;
            if new_pos == *old_pos {
                continue;
            }
            sqlx::query(
                "UPDATE agent_skills SET position = $3 WHERE agent_id = $1 AND skill_id = $2",
            )
            .bind(agent_id)
            .bind(sid)
            .bind(new_pos)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn compose(&self, agent_id: Uuid) -> AppResult<ComposePreview> {
        let agent: Option<(String, String, String)> = sqlx::query_as(
            "SELECT system_prompt, provider, model FROM agents WHERE id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;
        let (system_prompt, provider, model) =
            agent.ok_or_else(|| AppError::NotFound(format!("agent {agent_id}")))?;

        let bodies: Vec<String> = sqlx::query_scalar(
            r#"SELECT s.body FROM agent_skills a
               JOIN skills s ON s.id = a.skill_id
               WHERE a.agent_id = $1
               ORDER BY a.position ASC"#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        let mut composed = system_prompt;
        for body in &bodies {
            composed.push_str("\n\n");
            composed.push_str(body);
        }

        let length_chars = composed.chars().count() as i64;
        let budget = model_context_chars(&provider, &model);
        let fraction = if budget > 0 {
            length_chars as f32 / budget as f32
        } else {
            0.0
        };
        let warning = if fraction >= WARN_FRACTION {
            Some(format!(
                "composed prompt is at {pct}% of model budget — consider shortening or detaching skills",
                pct = (fraction * 100.0).round() as i64,
            ))
        } else {
            None
        };

        Ok(ComposePreview {
            composed,
            length_chars,
            model_context_chars: budget,
            fraction,
            warning,
        })
    }

    fn validate(input: &AttachReplaceInput) -> AppResult<()> {
        if input.skill_ids.len() > MAX_ATTACHED {
            return Err(AppError::validation_field(
                "skill_ids",
                format!("an agent can have at most {MAX_ATTACHED} attached skills"),
            ));
        }
        let mut seen = HashSet::with_capacity(input.skill_ids.len());
        for id in &input.skill_ids {
            if !seen.insert(*id) {
                return Err(AppError::validation_field(
                    "skill_ids",
                    "duplicate skill id in attachment list",
                ));
            }
        }
        Ok(())
    }

    async fn ensure_agent_exists(&self, agent_id: Uuid) -> AppResult<()> {
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM agents WHERE id = $1")
            .bind(agent_id)
            .fetch_optional(&self.pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!("agent {agent_id}")));
        }
        Ok(())
    }

    async fn ensure_skills_exist(&self, ids: &[Uuid]) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let found: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM skills WHERE id = ANY($1)")
            .bind(ids)
            .fetch_all(&self.pool)
            .await?;
        let found_set: HashSet<Uuid> = found.into_iter().collect();
        for id in ids {
            if !found_set.contains(id) {
                return Err(AppError::validation_field(
                    "skill_ids",
                    format!("skill {id} does not exist"),
                ));
            }
        }
        Ok(())
    }

    async fn compose_warnings(&self, agent_id: Uuid) -> AppResult<Vec<Warning>> {
        let preview = self.compose(agent_id).await?;
        let mut warnings = Vec::new();
        if let Some(message) = preview.warning {
            warnings.push(Warning { field: "composed", message });
        }
        Ok(warnings)
    }
}

/// Static per-(provider, model) context budget expressed in *characters*.
///
/// The conversion factor is a deliberate conservative approximation of ~4 chars per
/// token. F07 will replace this with a real tokenizer; for F04 it only drives a UI
/// warning chip, never blocks a save.
pub fn model_context_chars(provider: &str, _model: &str) -> i64 {
    let p = ProviderName::from_str(provider).ok();
    match p {
        Some(ProviderName::Anthropic) => 200_000 * 4,
        // Conservative default covering the gpt-4o and o1 families.
        Some(ProviderName::OpenAi) => 128_000 * 4,
        Some(ProviderName::OpenAiCompat) => 32_000 * 4,
        None => 32_000 * 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_more_than_twenty_skills() {
        let input = AttachReplaceInput {
            skill_ids: (0..21).map(|_| Uuid::new_v4()).collect(),
        };
        assert!(matches!(
            AttachmentsService::validate(&input),
            Err(AppError::Validation { .. })
        ));
    }

    #[test]
    fn accepts_exactly_twenty_skills() {
        let input = AttachReplaceInput {
            skill_ids: (0..20).map(|_| Uuid::new_v4()).collect(),
        };
        assert!(AttachmentsService::validate(&input).is_ok());
    }

    #[test]
    fn rejects_duplicate_skill_ids() {
        let dup = Uuid::new_v4();
        let input = AttachReplaceInput {
            skill_ids: vec![dup, Uuid::new_v4(), dup],
        };
        assert!(matches!(
            AttachmentsService::validate(&input),
            Err(AppError::Validation { .. })
        ));
    }

    #[test]
    fn accepts_empty_attachment_list() {
        let input = AttachReplaceInput { skill_ids: vec![] };
        assert!(AttachmentsService::validate(&input).is_ok());
    }

    #[test]
    fn model_context_anthropic_default() {
        assert_eq!(model_context_chars("anthropic", "claude-opus-4-7"), 800_000);
    }

    #[test]
    fn model_context_openai_default() {
        assert_eq!(model_context_chars("openai", "gpt-4o"), 512_000);
    }

    #[test]
    fn model_context_openai_compat_default() {
        assert_eq!(model_context_chars("openai_compat", "llama3"), 128_000);
    }

    #[test]
    fn model_context_unknown_provider_falls_back() {
        assert_eq!(model_context_chars("nonsense", "foo"), 128_000);
    }
}
