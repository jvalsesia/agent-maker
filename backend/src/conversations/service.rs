use super::model::*;
use crate::error::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct ConversationsService {
    pub pool: PgPool,
}

impl ConversationsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Return all conversations for an agent, ordered by last-activity DESC.
    /// If the agent has zero conversations, transactionally create one
    /// placeholder titled "New conversation" and return it as a singleton.
    pub async fn list_for_agent(&self, agent_id: Uuid) -> AppResult<Vec<Conversation>> {
        let mut tx = self.pool.begin().await?;

        let agent_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM agents WHERE id = $1")
                .bind(agent_id)
                .fetch_optional(&mut *tx)
                .await?;
        if agent_exists.is_none() {
            return Err(AppError::NotFound(format!("agent {agent_id}")));
        }

        let rows: Vec<Conversation> = sqlx::query_as(
            r#"SELECT id, agent_id, title, created_at, last_activity_at, message_count
               FROM conversations
               WHERE agent_id = $1
               ORDER BY last_activity_at DESC"#,
        )
        .bind(agent_id)
        .fetch_all(&mut *tx)
        .await?;

        if !rows.is_empty() {
            tx.commit().await?;
            return Ok(rows);
        }

        // Auto-create the singleton placeholder.
        let inserted: Conversation = sqlx::query_as(
            r#"INSERT INTO conversations (agent_id, title)
               VALUES ($1, $2)
               RETURNING id, agent_id, title, created_at, last_activity_at, message_count"#,
        )
        .bind(agent_id)
        .bind(DEFAULT_TITLE)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(vec![inserted])
    }

    pub async fn get(&self, id: Uuid) -> AppResult<Conversation> {
        let row: Option<Conversation> = sqlx::query_as(
            r#"SELECT id, agent_id, title, created_at, last_activity_at, message_count
               FROM conversations WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| AppError::NotFound(format!("conversation {id}")))
    }

    pub async fn create(
        &self,
        agent_id: Uuid,
        input: CreateInput,
    ) -> AppResult<Conversation> {
        let title = match input.title {
            Some(t) => {
                let trimmed = t.trim().to_string();
                validate_title(&trimmed)?;
                trimmed
            }
            None => DEFAULT_TITLE.to_string(),
        };

        // Verify parent agent exists (FK would reject too, but this gives a 404).
        let agent_exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM agents WHERE id = $1")
                .bind(agent_id)
                .fetch_optional(&self.pool)
                .await?;
        if agent_exists.is_none() {
            return Err(AppError::NotFound(format!("agent {agent_id}")));
        }

        let row: Conversation = sqlx::query_as(
            r#"INSERT INTO conversations (agent_id, title)
               VALUES ($1, $2)
               RETURNING id, agent_id, title, created_at, last_activity_at, message_count"#,
        )
        .bind(agent_id)
        .bind(&title)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn rename(&self, id: Uuid, input: RenameInput) -> AppResult<Conversation> {
        let trimmed = input.title.trim().to_string();
        validate_title(&trimmed)?;

        let row: Option<Conversation> = sqlx::query_as(
            r#"UPDATE conversations
               SET title = $1, last_activity_at = now()
               WHERE id = $2
               RETURNING id, agent_id, title, created_at, last_activity_at, message_count"#,
        )
        .bind(&trimmed)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| AppError::NotFound(format!("conversation {id}")))
    }

    pub async fn delete(&self, id: Uuid) -> AppResult<()> {
        let res = sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("conversation {id}")));
        }
        Ok(())
    }

    pub async fn list_messages(&self, id: Uuid) -> AppResult<ListMessagesResponse> {
        let conversation = self.get(id).await?;
        let messages: Vec<Message> = sqlx::query_as(
            r#"SELECT id, conversation_id, role, content, status, model, token_count, created_at
               FROM messages
               WHERE conversation_id = $1
               ORDER BY created_at ASC, id ASC"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(ListMessagesResponse { conversation, messages })
    }
}

fn validate_title(title: &str) -> AppResult<()> {
    let len = title.chars().count();
    if len < TITLE_MIN {
        return Err(AppError::validation_field("title", "Title cannot be empty"));
    }
    if len > TITLE_MAX {
        return Err(AppError::validation_field(
            "title",
            format!("Title must be {TITLE_MAX} characters or fewer"),
        ));
    }
    Ok(())
}
