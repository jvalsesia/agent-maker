use std::sync::Arc;

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::embedding::{EmbeddingError, EmbeddingProvider};
use super::model::*;
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct MemoryService {
    pub pool: PgPool,
    pub embedder: Arc<dyn EmbeddingProvider>,
}

impl MemoryService {
    pub fn new(pool: PgPool, embedder: Arc<dyn EmbeddingProvider>) -> Self {
        Self { pool, embedder }
    }

    /// Resolve effective (N, K) by merging per-request override → agent override → global default.
    /// Returns the parent agent_id along with the values.
    pub async fn resolve_n_k(
        &self,
        conversation_id: Uuid,
        override_n: Option<i16>,
        override_k: Option<i16>,
    ) -> AppResult<(Uuid, i16, i16)> {
        // Load agent + globals in one round-trip.
        let row = sqlx::query(
            r#"SELECT c.agent_id AS agent_id,
                      a.recent_n_override AS a_n,
                      a.top_k_override    AS a_k,
                      s.recent_n          AS g_n,
                      s.top_k             AS g_k
               FROM conversations c
               JOIN agents a ON a.id = c.agent_id
               CROSS JOIN settings s
               WHERE c.id = $1 AND s.id = 'singleton'"#,
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or_else(|| {
            AppError::NotFound(format!("conversation {conversation_id}"))
        })?;
        let agent_id: Uuid = row.get("agent_id");
        let a_n: Option<i16> = row.try_get("a_n").ok().flatten();
        let a_k: Option<i16> = row.try_get("a_k").ok().flatten();
        let g_n: i16 = row.get("g_n");
        let g_k: i16 = row.get("g_k");

        let n = override_n.or(a_n).unwrap_or(g_n);
        let k = override_k.or(a_k).unwrap_or(g_k);

        if !(RECENT_N_MIN..=RECENT_N_MAX).contains(&n) {
            return Err(AppError::validation_field(
                "recent_n",
                format!("must be in [{RECENT_N_MIN},{RECENT_N_MAX}]"),
            ));
        }
        if !(TOP_K_MIN..=TOP_K_MAX).contains(&k) {
            return Err(AppError::validation_field(
                "top_k",
                format!("must be in [{TOP_K_MIN},{TOP_K_MAX}]"),
            ));
        }
        Ok((agent_id, n, k))
    }

    /// Embed every user/assistant message in the conversation that lacks a row in
    /// `message_embeddings`. Each embed happens in its own try/catch; failures are counted.
    pub async fn embed_pending(&self, conversation_id: Uuid) -> AppResult<EmbedReport> {
        // Verify conversation exists.
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM conversations WHERE id = $1")
                .bind(conversation_id)
                .fetch_optional(&self.pool)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!(
                "conversation {conversation_id}"
            )));
        }

        // Total user/assistant messages and already-embedded count, for skipped reporting.
        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM messages
               WHERE conversation_id = $1 AND role IN ('user','assistant')"#,
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;

        let pending: Vec<(Uuid, String)> = sqlx::query_as(
            r#"SELECT m.id, m.content
               FROM messages m
               LEFT JOIN message_embeddings e ON e.message_id = m.id
               WHERE m.conversation_id = $1
                 AND m.role IN ('user','assistant')
                 AND e.message_id IS NULL
               ORDER BY m.created_at ASC"#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let skipped_already_present = (total as usize).saturating_sub(pending.len());
        let mut embedded = 0usize;
        let mut failed = 0usize;

        for (msg_id, content) in pending {
            match self.embedder.embed_one(&content).await {
                Ok(vec) => {
                    if vec.len() != EMBEDDING_DIM as usize {
                        failed += 1;
                        continue;
                    }
                    let vec_lit = vector_literal(&vec);
                    let res = sqlx::query(
                        r#"INSERT INTO message_embeddings
                             (message_id, conversation_id, model, dim, embedding)
                           VALUES ($1, $2, $3, $4, $5::vector)
                           ON CONFLICT (message_id) DO NOTHING"#,
                    )
                    .bind(msg_id)
                    .bind(conversation_id)
                    .bind(EMBEDDING_MODEL)
                    .bind(EMBEDDING_DIM)
                    .bind(&vec_lit)
                    .execute(&self.pool)
                    .await;
                    match res {
                        Ok(_) => embedded += 1,
                        Err(e) => {
                            tracing::warn!(
                                msg_id = %msg_id,
                                error = %e,
                                "embedding insert failed"
                            );
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(msg_id = %msg_id, error = %e, "embedding call failed");
                    failed += 1;
                }
            }
        }

        Ok(EmbedReport { embedded, skipped_already_present, failed })
    }

    /// Compose memory block: recent-N tail (oldest first) + top-K similar older turns.
    pub async fn query(
        &self,
        conversation_id: Uuid,
        body: MemoryQuery,
    ) -> AppResult<MemoryBlock> {
        let query = body.query.trim().to_string();
        if query.is_empty() {
            return Err(AppError::validation_field("query", "must not be empty"));
        }
        if query.chars().count() > QUERY_MAX_CHARS {
            return Err(AppError::validation_field(
                "query",
                format!("must be at most {QUERY_MAX_CHARS} characters"),
            ));
        }

        let (agent_id, n, k) =
            self.resolve_n_k(conversation_id, body.recent_n, body.top_k).await?;

        // Pull recent tail (newest first via DESC), then reverse to oldest-first for the response.
        let recent_desc: Vec<(Uuid, String, String, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT id, role, content, created_at
               FROM messages
               WHERE conversation_id = $1
                 AND role IN ('user','assistant')
               ORDER BY created_at DESC, id DESC
               LIMIT $2"#,
        )
        .bind(conversation_id)
        .bind(n as i64)
        .fetch_all(&self.pool)
        .await?;
        let recent_ids: Vec<Uuid> = recent_desc.iter().map(|r| r.0).collect();
        let mut recent: Vec<MemoryTurn> = recent_desc
            .into_iter()
            .map(|(id, role, content, created_at)| MemoryTurn {
                id,
                role,
                content,
                created_at,
            })
            .collect();
        recent.reverse();

        let mut block = MemoryBlock {
            conversation_id,
            agent_id,
            effective_n: n,
            effective_k: k,
            recent,
            retrieved: Vec::new(),
            degraded: false,
            degraded_reason: None,
        };

        if k == 0 {
            return Ok(block);
        }

        let query_vec = match self.embedder.embed_one(&query).await {
            Ok(v) if v.len() == EMBEDDING_DIM as usize => v,
            Ok(_) => {
                block.degraded = true;
                block.degraded_reason = Some("embedding_dim_mismatch".into());
                return Ok(block);
            }
            Err(EmbeddingError::NoKey) => {
                block.degraded = true;
                block.degraded_reason = Some("no_openai_key".into());
                return Ok(block);
            }
            Err(e) => {
                tracing::warn!(error = %e, "embedding query failed");
                block.degraded = true;
                block.degraded_reason = Some("embedding_error".into());
                return Ok(block);
            }
        };
        let vec_lit = vector_literal(&query_vec);

        let res = sqlx::query(
            r#"SELECT m.id, m.role, m.content, m.created_at,
                      1.0 - (e.embedding <=> $1::vector) AS similarity
               FROM message_embeddings e
               JOIN messages m ON m.id = e.message_id
               WHERE e.conversation_id = $2
                 AND NOT (m.id = ANY($3))
               ORDER BY e.embedding <=> $1::vector ASC
               LIMIT $4"#,
        )
        .bind(&vec_lit)
        .bind(conversation_id)
        .bind(&recent_ids)
        .bind(k as i64)
        .fetch_all(&self.pool)
        .await;

        match res {
            Ok(rows) => {
                let retrieved = rows
                    .into_iter()
                    .map(|r| {
                        let sim: f64 = r.try_get::<f64, _>("similarity").unwrap_or(0.0);
                        RetrievedTurn {
                            id: r.get("id"),
                            role: r.get("role"),
                            content: r.get("content"),
                            created_at: r.get("created_at"),
                            similarity: sim as f32,
                        }
                    })
                    .collect();
                block.retrieved = retrieved;
            }
            Err(e) => {
                tracing::warn!(error = %e, "pgvector retrieval failed");
                block.degraded = true;
                block.degraded_reason = Some("retrieval_error".into());
            }
        }

        Ok(block)
    }

    /// Returns the number of embedded turns currently stored for the conversation.
    pub async fn stats(&self, conversation_id: Uuid) -> AppResult<i64> {
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM conversations WHERE id = $1")
                .bind(conversation_id)
                .fetch_optional(&self.pool)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!(
                "conversation {conversation_id}"
            )));
        }
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM message_embeddings WHERE conversation_id = $1",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    pub async fn clear(&self, conversation_id: Uuid) -> AppResult<ClearReport> {
        let exists: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM conversations WHERE id = $1")
                .bind(conversation_id)
                .fetch_optional(&self.pool)
                .await?;
        if exists.is_none() {
            return Err(AppError::NotFound(format!(
                "conversation {conversation_id}"
            )));
        }
        let res = sqlx::query("DELETE FROM message_embeddings WHERE conversation_id = $1")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        Ok(ClearReport { removed: res.rows_affected() as usize })
    }
}

/// Renders an f32 vector as the pgvector text literal `'[v1,v2,...]'` (without quotes).
fn vector_literal(v: &[f32]) -> String {
    let mut s = String::with_capacity(v.len() * 8 + 2);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        // Use f32 default formatting; finite values only.
        if x.is_finite() {
            s.push_str(&format!("{x}"));
        } else {
            s.push('0');
        }
    }
    s.push(']');
    s
}
