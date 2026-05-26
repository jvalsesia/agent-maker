use super::model::*;
use crate::{
    error::{AppError, AppResult},
    llm::{ProviderName, provider::mask_key},
    secrets::{AnyStore, SecretError, SecretStore},
};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

const NAME_MAX: usize = 60;
const PREAMBLE_MAX: usize = 500;
const SYSTEM_PROMPT_MAX: usize = 20_000;
const SYSTEM_PROMPT_SOFT_MIN: usize = 50;
const SYSTEM_PROMPT_SOFT_MAX: usize = 10_000;

#[derive(Clone)]
pub struct AgentsService {
    pub pool: PgPool,
    pub secrets: Arc<AnyStore>,
}

impl AgentsService {
    pub fn new(pool: PgPool, secrets: Arc<AnyStore>) -> Self {
        Self { pool, secrets }
    }

    pub fn validate(input: &AgentUpsert) -> AppResult<Vec<Warning>> {
        let name = input.name.trim();
        if name.is_empty() || name.chars().count() > NAME_MAX {
            return Err(AppError::validation_field(
                "name",
                "must be 1–60 characters after trimming",
            ));
        }
        if let Some(p) = &input.preamble {
            if p.chars().count() > PREAMBLE_MAX {
                return Err(AppError::validation_field(
                    "preamble",
                    "must be at most 500 characters",
                ));
            }
        }
        if input.system_prompt.is_empty() {
            return Err(AppError::validation_field("system_prompt", "is required"));
        }
        if input.system_prompt.chars().count() > SYSTEM_PROMPT_MAX {
            return Err(AppError::validation_field(
                "system_prompt",
                "must be at most 20000 characters",
            ));
        }
        if input.provider.parse::<ProviderName>().is_err() {
            return Err(AppError::validation_field(
                "provider",
                "must be one of anthropic, openai, openai_compat",
            ));
        }
        if input.model.trim().is_empty() {
            return Err(AppError::validation_field("model", "is required"));
        }
        if let Some(n) = input.recent_n_override {
            if !(4..=30).contains(&n) {
                return Err(AppError::validation_field(
                    "recent_n_override",
                    "must be in [4,30]",
                ));
            }
        }
        if let Some(k) = input.top_k_override {
            if !(0..=10).contains(&k) {
                return Err(AppError::validation_field(
                    "top_k_override",
                    "must be in [0,10]",
                ));
            }
        }
        if !crate::i18n::is_valid_response_language(&input.response_language) {
            return Err(AppError::validation_field(
                "response_language",
                "must be auto, en, or pt-BR",
            ));
        }

        let mut warnings = Vec::new();
        let sp_len = input.system_prompt.chars().count();
        if sp_len < SYSTEM_PROMPT_SOFT_MIN {
            warnings.push(Warning {
                field: "system_prompt",
                message: "shorter than 50 characters — the agent may behave inconsistently",
            });
        } else if sp_len > SYSTEM_PROMPT_SOFT_MAX {
            warnings.push(Warning {
                field: "system_prompt",
                message: "longer than 10000 characters — may crowd the model's context window",
            });
        }
        Ok(warnings)
    }

    pub async fn list(&self, q: ListQuery) -> AppResult<Vec<AgentSummary>> {
        let sort = match q.sort.unwrap_or_default() {
            SortField::Name => "a.name",
            SortField::LastUsed => "a.last_used_at",
            SortField::Created => "a.created_at",
        };
        let order = match q.order.unwrap_or_default() {
            SortOrder::Asc => "ASC NULLS LAST",
            SortOrder::Desc => "DESC NULLS LAST",
        };
        let search = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty());

        // Parameterized search via ILIKE; sort is whitelisted above so direct interpolation is safe.
        let base = format!(
            r#"SELECT a.id, a.name, a.preamble, a.provider, a.model, a.has_override_key,
                      a.last_used_at, a.created_at,
                      COALESCE(s.cnt, 0) AS attached_skill_count,
                      COALESCE(c.cnt, 0) AS conversation_count
               FROM agents a
               LEFT JOIN (SELECT agent_id, COUNT(*) AS cnt FROM agent_skills GROUP BY agent_id) s
                   ON s.agent_id = a.id
               LEFT JOIN (SELECT agent_id, COUNT(*) AS cnt FROM conversations GROUP BY agent_id) c
                   ON c.agent_id = a.id
               {where_clause}
               ORDER BY {sort} {order}, a.name ASC"#,
            where_clause = if search.is_some() {
                "WHERE a.name ILIKE $1 OR a.preamble ILIKE $1"
            } else {
                ""
            },
        );

        let rows = if let Some(s) = search {
            sqlx::query_as::<_, AgentSummary>(&base)
                .bind(format!("%{s}%"))
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, AgentSummary>(&base)
                .fetch_all(&self.pool)
                .await?
        };
        Ok(rows)
    }

    pub async fn get(&self, id: Uuid) -> AppResult<Agent> {
        let agent: Option<Agent> = sqlx::query_as(
            r#"SELECT id, name, preamble, system_prompt, provider, model, has_override_key,
                      recent_n_override, top_k_override, response_language,
                      created_at, updated_at, last_used_at
               FROM agents WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        agent.ok_or_else(|| AppError::NotFound(format!("agent {id}")))
    }

    pub async fn create(&self, input: AgentUpsert) -> AppResult<AgentResponse> {
        let warnings = Self::validate(&input)?;
        let row = sqlx::query_as::<_, Agent>(
            r#"INSERT INTO agents
                 (name, preamble, system_prompt, provider, model, recent_n_override, top_k_override,
                  response_language)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id, name, preamble, system_prompt, provider, model, has_override_key,
                         recent_n_override, top_k_override, response_language,
                         created_at, updated_at, last_used_at"#,
        )
        .bind(input.name.trim())
        .bind(input.preamble.as_deref())
        .bind(&input.system_prompt)
        .bind(&input.provider)
        .bind(&input.model)
        .bind(input.recent_n_override)
        .bind(input.top_k_override)
        .bind(&input.response_language)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        Ok(AgentResponse { agent: row, warnings })
    }

    pub async fn update(&self, id: Uuid, input: AgentUpsert) -> AppResult<AgentResponse> {
        let warnings = Self::validate(&input)?;
        let row: Option<Agent> = sqlx::query_as(
            r#"UPDATE agents
               SET name=$2, preamble=$3, system_prompt=$4, provider=$5, model=$6,
                   recent_n_override=$7, top_k_override=$8, response_language=$9
               WHERE id=$1
               RETURNING id, name, preamble, system_prompt, provider, model, has_override_key,
                         recent_n_override, top_k_override, response_language,
                         created_at, updated_at, last_used_at"#,
        )
        .bind(id)
        .bind(input.name.trim())
        .bind(input.preamble.as_deref())
        .bind(&input.system_prompt)
        .bind(&input.provider)
        .bind(&input.model)
        .bind(input.recent_n_override)
        .bind(input.top_k_override)
        .bind(&input.response_language)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        let agent = row.ok_or_else(|| AppError::NotFound(format!("agent {id}")))?;
        Ok(AgentResponse { agent, warnings })
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let existing = self.get(id).await?;
        // Remove the namespaced override secret first; idempotent if not present.
        let slug = key_slug(id, &existing.provider);
        match self.secrets.delete(&slug).await {
            Ok(()) | Err(SecretError::NotFound) => {}
            Err(e) => return Err(AppError::KeyStore(e.to_string())),
        }
        let n = sqlx::query("DELETE FROM agents WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(AppError::NotFound(format!("agent {id}")));
        }
        Ok(())
    }

    pub async fn clone_agent(&self, id: Uuid, req: CloneRequest) -> AppResult<AgentResponse> {
        let src = self.get(id).await?;
        let base_name = req
            .name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{} (copy)", src.name));
        let final_name = self.unique_name(&base_name).await?;

        let row = sqlx::query_as::<_, Agent>(
            r#"INSERT INTO agents
                 (name, preamble, system_prompt, provider, model, recent_n_override, top_k_override,
                  response_language)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id, name, preamble, system_prompt, provider, model, has_override_key,
                         recent_n_override, top_k_override, response_language,
                         created_at, updated_at, last_used_at"#,
        )
        .bind(&final_name)
        .bind(src.preamble.as_deref())
        .bind(&src.system_prompt)
        .bind(&src.provider)
        .bind(&src.model)
        .bind(src.recent_n_override)
        .bind(src.top_k_override)
        .bind(&src.response_language)
        .fetch_one(&self.pool)
        .await?;
        Ok(AgentResponse { agent: row, warnings: Vec::new() })
    }

    pub async fn save_key(&self, id: Uuid, key: &str) -> AppResult<KeySaved> {
        let agent = self.get(id).await?;
        let slug = key_slug(id, &agent.provider);
        self.secrets
            .put(&slug, key)
            .await
            .map_err(|e| AppError::KeyStore(e.to_string()))?;
        sqlx::query("UPDATE agents SET has_override_key = TRUE WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(KeySaved { has_override_key: true, key_masked: mask_key(key) })
    }

    pub async fn delete_key(&self, id: Uuid) -> AppResult<()> {
        let agent = self.get(id).await?;
        let slug = key_slug(id, &agent.provider);
        match self.secrets.delete(&slug).await {
            Ok(()) | Err(SecretError::NotFound) => {}
            Err(e) => return Err(AppError::KeyStore(e.to_string())),
        }
        sqlx::query("UPDATE agents SET has_override_key = FALSE WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn unique_name(&self, base: &str) -> AppResult<String> {
        // Cheap probe: most clones land on the first attempt.
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM agents WHERE name = $1 OR name LIKE $2",
        )
        .bind(base)
        .bind(format!("{base} (%)"))
        .fetch_all(&self.pool)
        .await?;
        if !rows.iter().any(|n| n == base) {
            return Ok(base.to_string());
        }
        for n in 2..1000 {
            let candidate = format!("{base} ({n})");
            if !rows.iter().any(|x| x == &candidate) {
                return Ok(candidate);
            }
        }
        Err(AppError::Internal("could not find unique clone name".into()))
    }
}

pub fn key_slug(id: Uuid, provider: &str) -> String {
    format!("agent:{id}:{provider}")
}

fn map_unique_violation(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(ref db) = e {
        if db.code().as_deref() == Some("23505") {
            return AppError::validation_field(
                "name",
                "an agent with this name already exists",
            );
        }
    }
    AppError::Database(e)
}

/// Static model lists per provider — surfaced via `GET /api/agents/models`.
/// Kept here (not on the `LlmProvider` trait) so the dropdown data path stays sync and
/// independent of whether a key is configured.
pub fn available_models(provider: ProviderName) -> &'static [&'static str] {
    match provider {
        ProviderName::Anthropic => &[
            "claude-opus-4-7",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
        ],
        ProviderName::OpenAi => &["gpt-4o", "gpt-4o-mini", "o1-mini"],
        ProviderName::OpenAiCompat => &["llama3", "mistral", "qwen2"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> AgentUpsert {
        AgentUpsert {
            name: "Editor".into(),
            preamble: Some("A sharp writing editor".into()),
            system_prompt: "You are a sharp, opinionated writing editor. Be specific.".into(),
            provider: "anthropic".into(),
            model: "claude-haiku-4-5".into(),
            recent_n_override: None,
            top_k_override: None,
            response_language: "auto".into(),
        }
    }

    #[test]
    fn accepts_valid_input() {
        let warnings = AgentsService::validate(&valid()).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn rejects_empty_name() {
        let mut v = valid();
        v.name = "   ".into();
        assert!(matches!(AgentsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_name_too_long() {
        let mut v = valid();
        v.name = "x".repeat(61);
        assert!(matches!(AgentsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_bad_provider() {
        let mut v = valid();
        v.provider = "claude".into();
        assert!(matches!(AgentsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_top_k_out_of_range() {
        let mut v = valid();
        v.top_k_override = Some(11);
        assert!(matches!(AgentsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_bad_response_language() {
        let mut v = valid();
        v.response_language = "fr".into();
        assert!(matches!(AgentsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn accepts_specific_response_language() {
        let mut v = valid();
        v.response_language = "pt-BR".into();
        assert!(AgentsService::validate(&v).is_ok());
    }

    #[test]
    fn warns_on_short_prompt() {
        let mut v = valid();
        v.system_prompt = "be helpful".into();
        let warnings = AgentsService::validate(&v).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].field, "system_prompt");
    }

    #[test]
    fn warns_on_long_prompt() {
        let mut v = valid();
        v.system_prompt = "x".repeat(10_001);
        let warnings = AgentsService::validate(&v).unwrap();
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn rejects_prompt_over_hard_max() {
        let mut v = valid();
        v.system_prompt = "x".repeat(20_001);
        assert!(matches!(AgentsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn key_slug_is_namespaced() {
        let id = Uuid::nil();
        assert_eq!(
            key_slug(id, "anthropic"),
            "agent:00000000-0000-0000-0000-000000000000:anthropic"
        );
    }

    #[test]
    fn available_models_is_non_empty_for_every_provider() {
        for p in crate::llm::provider::ALL_PROVIDERS {
            assert!(!available_models(p).is_empty());
        }
    }
}
