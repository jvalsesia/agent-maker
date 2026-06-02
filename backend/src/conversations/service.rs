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
        let mut messages: Vec<Message> = sqlx::query_as(
            r#"SELECT id, conversation_id, role, content, status, model, token_count,
                      finish_reason, subagent_alias, subagent_agent_id, created_at
               FROM messages
               WHERE conversation_id = $1
               ORDER BY created_at ASC, id ASC"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        // Attach recalled turns to their assistant messages (one query, grouped in memory).
        let recalls: Vec<(Uuid, Uuid, String, String, chrono::DateTime<chrono::Utc>, f32)> =
            sqlx::query_as(
                r#"SELECT r.assistant_message_id, r.recalled_message_id, m.role, m.content,
                          m.created_at, r.similarity
                   FROM message_recalls r
                   JOIN messages m ON m.id = r.recalled_message_id
                   WHERE r.assistant_message_id IN (
                       SELECT id FROM messages WHERE conversation_id = $1
                   )
                   ORDER BY r.assistant_message_id, r.position ASC"#,
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await?;

        if !recalls.is_empty() {
            use std::collections::HashMap;
            let mut by_assistant: HashMap<Uuid, Vec<RecalledRef>> = HashMap::new();
            for (amid, message_id, role, content, created_at, similarity) in recalls {
                by_assistant
                    .entry(amid)
                    .or_default()
                    .push(RecalledRef { message_id, role, content, created_at, similarity });
            }
            for msg in &mut messages {
                if let Some(rs) = by_assistant.remove(&msg.id) {
                    msg.recalled = rs;
                }
            }
        }

        Ok(ListMessagesResponse { conversation, messages })
    }

    /// Insert a message and bump the conversation's activity/count in one
    /// transaction. The first user message also seeds the conversation title
    /// (F06 auto-title rule) when the title is still the default placeholder.
    pub async fn insert_message(
        &self,
        conversation_id: Uuid,
        m: NewMessage<'_>,
    ) -> AppResult<Message> {
        let mut tx = self.pool.begin().await?;

        let convo: Option<(String, i32)> = sqlx::query_as(
            "SELECT title, message_count FROM conversations WHERE id = $1 FOR UPDATE",
        )
        .bind(conversation_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (title, count) =
            convo.ok_or_else(|| AppError::NotFound(format!("conversation {conversation_id}")))?;

        let inserted: Message = sqlx::query_as(
            r#"INSERT INTO messages
                 (conversation_id, role, content, status, model, token_count, finish_reason,
                  subagent_alias, subagent_agent_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id, conversation_id, role, content, status, model, token_count,
                         finish_reason, subagent_alias, subagent_agent_id, created_at"#,
        )
        .bind(conversation_id)
        .bind(m.role)
        .bind(m.content)
        .bind(m.status)
        .bind(m.model)
        .bind(m.token_count)
        .bind(m.finish_reason)
        .bind(m.subagent_alias)
        .bind(m.subagent_agent_id)
        .fetch_one(&mut *tx)
        .await?;

        let new_title = if m.role == "user"
            && count == 0
            && title == DEFAULT_TITLE
            && !m.content.trim().is_empty()
        {
            Some(auto_title(m.content))
        } else {
            None
        };

        match new_title {
            Some(t) => {
                sqlx::query(
                    "UPDATE conversations
                     SET message_count = message_count + 1, last_activity_at = now(), title = $2
                     WHERE id = $1",
                )
                .bind(conversation_id)
                .bind(&t)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query(
                    "UPDATE conversations
                     SET message_count = message_count + 1, last_activity_at = now()
                     WHERE id = $1",
                )
                .bind(conversation_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(inserted)
    }

    /// Finalize a streamed message (content + status + tokens + finish reason).
    pub async fn update_message(
        &self,
        id: Uuid,
        content: &str,
        status: &str,
        token_count: Option<i32>,
        finish_reason: Option<&str>,
    ) -> AppResult<Message> {
        let row: Option<Message> = sqlx::query_as(
            r#"UPDATE messages
               SET content = $2, status = $3, token_count = $4, finish_reason = $5
               WHERE id = $1
               RETURNING id, conversation_id, role, content, status, model, token_count,
                         finish_reason, subagent_alias, subagent_agent_id, created_at"#,
        )
        .bind(id)
        .bind(content)
        .bind(status)
        .bind(token_count)
        .bind(finish_reason)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or_else(|| AppError::NotFound(format!("message {id}")))
    }

    /// The most recent message in a conversation (by created_at, id tiebreak).
    pub async fn last_message(&self, conversation_id: Uuid) -> AppResult<Option<Message>> {
        let row: Option<Message> = sqlx::query_as(
            r#"SELECT id, conversation_id, role, content, status, model, token_count,
                      finish_reason, subagent_alias, subagent_agent_id, created_at
               FROM messages
               WHERE conversation_id = $1
               ORDER BY created_at DESC, id DESC
               LIMIT 1"#,
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Delete a single message (used to supersede an errored/stopped assistant
    /// turn on retry); its recalls cascade away.
    pub async fn delete_message(&self, id: Uuid) -> AppResult<()> {
        sqlx::query("DELETE FROM messages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Persist the recalled-turn references for an assistant message.
    pub async fn insert_recalls(
        &self,
        assistant_message_id: Uuid,
        recalled: &[(Uuid, f32)],
    ) -> AppResult<()> {
        if recalled.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for (position, (recalled_id, similarity)) in recalled.iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO message_recalls
                     (assistant_message_id, recalled_message_id, similarity, position)
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT (assistant_message_id, recalled_message_id) DO NOTHING"#,
            )
            .bind(assistant_message_id)
            .bind(recalled_id)
            .bind(*similarity)
            .bind(position as i32)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

/// Truncate a first user message into a conversation title (≤ 60 chars).
fn auto_title(content: &str) -> String {
    content.trim().chars().take(AUTO_TITLE_MAX).collect()
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
