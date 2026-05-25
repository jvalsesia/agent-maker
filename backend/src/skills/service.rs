use super::model::*;
use crate::error::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

const NAME_MAX: usize = 60;
const DESCRIPTION_MAX: usize = 200;
const BODY_MAX: usize = 10_000;
const BODY_SOFT_MIN: usize = 50;
const BODY_SOFT_MAX: usize = 5_000;

#[derive(Clone)]
pub struct SkillsService {
    pub pool: PgPool,
}

impl SkillsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn validate(input: &SkillUpsert) -> AppResult<Vec<Warning>> {
        let name = input.name.trim();
        if name.is_empty() || name.chars().count() > NAME_MAX {
            return Err(AppError::validation_field(
                "name",
                "must be 1–60 characters after trimming",
            ));
        }
        let desc = input.description.trim();
        if desc.is_empty() || desc.chars().count() > DESCRIPTION_MAX {
            return Err(AppError::validation_field(
                "description",
                "must be 1–200 characters after trimming",
            ));
        }
        if input.body.is_empty() {
            return Err(AppError::validation_field("body", "is required"));
        }
        if input.body.chars().count() > BODY_MAX {
            return Err(AppError::validation_field(
                "body",
                "must be at most 10000 characters",
            ));
        }

        let mut warnings = Vec::new();
        let body_len = input.body.chars().count();
        if body_len < BODY_SOFT_MIN {
            warnings.push(Warning {
                field: "body",
                message: "shorter than 50 characters — the skill may be too thin to be useful",
            });
        } else if body_len > BODY_SOFT_MAX {
            warnings.push(Warning {
                field: "body",
                message: "longer than 5000 characters — may crowd composed prompts",
            });
        }
        Ok(warnings)
    }

    pub async fn list(&self, q: ListQuery) -> AppResult<Vec<SkillSummary>> {
        let sort = match q.sort.unwrap_or_default() {
            SortField::Name => "s.name",
            SortField::Attached => "attached_agent_count",
            SortField::Created => "s.created_at",
        };
        let order = match q.order.unwrap_or_default() {
            SortOrder::Asc => "ASC NULLS LAST",
            SortOrder::Desc => "DESC NULLS LAST",
        };
        let search = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty());

        let base = format!(
            r#"SELECT s.id, s.name, s.description, s.created_at, s.updated_at,
                      COALESCE(a.cnt, 0) AS attached_agent_count
               FROM skills s
               LEFT JOIN (SELECT skill_id, COUNT(*) AS cnt FROM agent_skills GROUP BY skill_id) a
                   ON a.skill_id = s.id
               {where_clause}
               ORDER BY {sort} {order}, s.name ASC"#,
            where_clause = if search.is_some() {
                "WHERE s.name ILIKE $1 OR s.description ILIKE $1"
            } else {
                ""
            },
        );

        let rows = if let Some(s) = search {
            sqlx::query_as::<_, SkillSummary>(&base)
                .bind(format!("%{s}%"))
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query_as::<_, SkillSummary>(&base)
                .fetch_all(&self.pool)
                .await?
        };
        Ok(rows)
    }

    pub async fn get(&self, id: Uuid) -> AppResult<Skill> {
        let skill: Option<Skill> = sqlx::query_as(
            r#"SELECT id, name, description, body, created_at, updated_at
               FROM skills WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        skill.ok_or_else(|| AppError::NotFound(format!("skill {id}")))
    }

    pub async fn get_detail(&self, id: Uuid) -> AppResult<SkillDetail> {
        let skill = self.get(id).await?;
        let using_agents: Vec<UsingAgent> = sqlx::query_as(
            r#"SELECT a.id, a.name
               FROM agents a
               JOIN agent_skills s ON s.agent_id = a.id
               WHERE s.skill_id = $1
               ORDER BY a.name ASC"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(SkillDetail { skill, using_agents })
    }

    pub async fn create(&self, input: SkillUpsert) -> AppResult<SkillResponse> {
        let warnings = Self::validate(&input)?;
        let row = sqlx::query_as::<_, Skill>(
            r#"INSERT INTO skills (name, description, body)
               VALUES ($1, $2, $3)
               RETURNING id, name, description, body, created_at, updated_at"#,
        )
        .bind(input.name.trim())
        .bind(input.description.trim())
        .bind(&input.body)
        .fetch_one(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        Ok(SkillResponse { skill: row, warnings })
    }

    pub async fn update(&self, id: Uuid, input: SkillUpsert) -> AppResult<SkillResponse> {
        let warnings = Self::validate(&input)?;
        let row: Option<Skill> = sqlx::query_as(
            r#"UPDATE skills
               SET name=$2, description=$3, body=$4
               WHERE id=$1
               RETURNING id, name, description, body, created_at, updated_at"#,
        )
        .bind(id)
        .bind(input.name.trim())
        .bind(input.description.trim())
        .bind(&input.body)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_unique_violation)?;
        let skill = row.ok_or_else(|| AppError::NotFound(format!("skill {id}")))?;
        Ok(SkillResponse { skill, warnings })
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let n = sqlx::query("DELETE FROM skills WHERE id=$1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(AppError::NotFound(format!("skill {id}")));
        }
        Ok(())
    }

    pub async fn clone_skill(&self, id: Uuid, req: CloneRequest) -> AppResult<SkillResponse> {
        let src = self.get(id).await?;
        let base_name = req
            .name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{} (copy)", src.name));
        let final_name = self.unique_name(&base_name).await?;

        let row = sqlx::query_as::<_, Skill>(
            r#"INSERT INTO skills (name, description, body)
               VALUES ($1, $2, $3)
               RETURNING id, name, description, body, created_at, updated_at"#,
        )
        .bind(&final_name)
        .bind(&src.description)
        .bind(&src.body)
        .fetch_one(&self.pool)
        .await?;
        Ok(SkillResponse { skill: row, warnings: Vec::new() })
    }

    async fn unique_name(&self, base: &str) -> AppResult<String> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM skills WHERE name = $1 OR name LIKE $2",
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

fn map_unique_violation(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(ref db) = e
        && db.code().as_deref() == Some("23505") {
            return AppError::validation_field(
                "name",
                "a skill with this name already exists",
            );
        }
    AppError::Database(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> SkillUpsert {
        SkillUpsert {
            name: "Concise Replies".into(),
            description: "Keep responses tight and skimmable".into(),
            body: "Prefer short sentences. Avoid filler. Use bullet lists when listing.".into(),
        }
    }

    #[test]
    fn accepts_valid_input() {
        let warnings = SkillsService::validate(&valid()).unwrap();
        assert!(warnings.is_empty());
    }

    #[test]
    fn rejects_empty_name() {
        let mut v = valid();
        v.name = "   ".into();
        assert!(matches!(SkillsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_name_too_long() {
        let mut v = valid();
        v.name = "x".repeat(61);
        assert!(matches!(SkillsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_empty_description() {
        let mut v = valid();
        v.description = " ".into();
        assert!(matches!(SkillsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_description_too_long() {
        let mut v = valid();
        v.description = "x".repeat(201);
        assert!(matches!(SkillsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_empty_body() {
        let mut v = valid();
        v.body = "".into();
        assert!(matches!(SkillsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn rejects_body_over_hard_max() {
        let mut v = valid();
        v.body = "x".repeat(10_001);
        assert!(matches!(SkillsService::validate(&v), Err(AppError::Validation { .. })));
    }

    #[test]
    fn warns_on_short_body() {
        let mut v = valid();
        v.body = "be brief".into();
        let warnings = SkillsService::validate(&v).unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].field, "body");
    }

    #[test]
    fn warns_on_long_body() {
        let mut v = valid();
        v.body = "x".repeat(5_001);
        let warnings = SkillsService::validate(&v).unwrap();
        assert_eq!(warnings.len(), 1);
    }
}
