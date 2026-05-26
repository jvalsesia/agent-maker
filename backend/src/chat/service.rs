use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use sqlx::PgPool;
use uuid::Uuid;

use super::compose::compose;
use super::model::{ChatStartRequest, RecalledTurn, StreamEvent};
use crate::agents::AgentsService;
use crate::agents::service::key_slug;
use crate::conversations::ConversationsService;
use crate::conversations::model::NewMessage;
use crate::error::{AppError, AppResult};
use crate::llm::{ChatError, ChatRequest, ChatUsage, ProviderName, ProviderRegistry};
use crate::memory::MemoryService;
use crate::memory::model::MemoryQuery;
use crate::secrets::{AnyStore, SecretStore};
use crate::skill_attachments::AttachmentsService;
use crate::skill_attachments::service::model_context_chars;

/// Default ceiling on generated tokens per turn, and the per-token char reserve
/// subtracted from the model budget to leave room for the response.
const DEFAULT_MAX_TOKENS: u32 = 4096;
const CHARS_PER_TOKEN: i64 = 4;
const SSE_BUFFER: usize = 64;

#[derive(Clone)]
pub struct ChatService {
    pub pool: PgPool,
    pub providers: ProviderRegistry,
    pub agents: AgentsService,
    pub conversations: ConversationsService,
    pub memory: MemoryService,
    pub attachments: AttachmentsService,
    pub secrets: Arc<AnyStore>,
    inflight: Arc<Mutex<HashSet<Uuid>>>,
}

/// Removes a conversation from the in-flight set when the stream task ends.
struct InflightGuard {
    set: Arc<Mutex<HashSet<Uuid>>>,
    id: Uuid,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = self.set.lock() {
            set.remove(&self.id);
        }
    }
}

impl ChatService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool: PgPool,
        providers: ProviderRegistry,
        agents: AgentsService,
        conversations: ConversationsService,
        memory: MemoryService,
        attachments: AttachmentsService,
        secrets: Arc<AnyStore>,
    ) -> Self {
        Self {
            pool,
            providers,
            agents,
            conversations,
            memory,
            attachments,
            secrets,
            inflight: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Run preflight (validate, compose, dispatch) and return a stream of SSE
    /// events. Errors here surface as HTTP responses before any token streams.
    pub async fn start(
        &self,
        conversation_id: Uuid,
        req: ChatStartRequest,
    ) -> AppResult<mpsc::Receiver<StreamEvent>> {
        // One in-flight response per conversation. Held for the whole stream;
        // released when the spawned task (or an early error here) drops it.
        let guard = self.acquire(conversation_id)?;

        // Resolve the agent behind this conversation.
        let agent_id: Uuid = sqlx::query_scalar("SELECT agent_id FROM conversations WHERE id = $1")
            .bind(conversation_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("conversation {conversation_id}")))?;
        let agent = self.agents.get(agent_id).await?;
        let provider_name = ProviderName::from_str(&agent.provider)
            .map_err(|e| AppError::Internal(format!("bad provider on agent: {e}")))?;

        // Determine the user query: a fresh message, or the trailing user
        // message reused on retry (superseding a prior errored/stopped reply).
        let (user_query, user_message_id) = self
            .resolve_user_message(conversation_id, &req)
            .await?;

        // F08 memory block for this query (recent tail includes the user turn).
        let block = self
            .memory
            .query(
                conversation_id,
                MemoryQuery { query: user_query.clone(), recent_n: req.recent_n, top_k: req.top_k },
            )
            .await?;

        // Base system = agent prompt + ordered skill bodies (F04 compose),
        // optionally prefixed with the F09 response-language directive.
        let composed_base = self.attachments.compose(agent_id).await?.composed;
        let base_system =
            apply_language_directive(&agent.response_language, req.locale.as_deref(), &composed_base);
        let budget = model_context_chars(&agent.provider, &agent.model);
        let reserve = DEFAULT_MAX_TOKENS as i64 * CHARS_PER_TOKEN;
        let composed = compose(&base_system, &block, budget, reserve)?;

        // Resolve the per-agent key override and the local base URL if any.
        let api_key = self.resolve_key(&agent).await;
        let base_url = self.resolve_base_url(&agent.provider).await;

        // Open the provider stream — provider 4xx/auth surfaces here as HTTP.
        let provider = self.providers.get(provider_name);
        let chat_req = ChatRequest {
            model: agent.model.clone(),
            system: Some(composed.system),
            messages: composed.messages,
            base_url,
            api_key,
            max_tokens: DEFAULT_MAX_TOKENS,
        };
        let delta_stream = provider
            .chat(chat_req)
            .await
            .map_err(|e| chat_error_to_app(e, &agent.provider))?;

        // Persist an assistant placeholder; attach the recalled turns to it.
        let assistant = self
            .conversations
            .insert_message(
                conversation_id,
                NewMessage::assistant("", "complete", Some(&agent.model)),
            )
            .await?;
        let recall_pairs: Vec<(Uuid, f32)> =
            block.retrieved.iter().map(|r| (r.id, r.similarity)).collect();
        self.conversations
            .insert_recalls(assistant.id, &recall_pairs)
            .await?;

        // Touch last-used so the agent sorts as recently active.
        let _ = sqlx::query("UPDATE agents SET last_used_at = now() WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await;

        let meta = StreamEvent::Meta {
            user_message_id,
            assistant_message_id: assistant.id,
            model: agent.model.clone(),
            degraded: block.degraded,
            degraded_reason: block.degraded_reason.clone(),
            recalled: block
                .retrieved
                .iter()
                .map(|r| RecalledTurn {
                    message_id: r.id,
                    role: r.role.clone(),
                    content: r.content.clone(),
                    created_at: r.created_at,
                    similarity: r.similarity,
                })
                .collect(),
        };

        // Spawn the streaming/persistence task; it owns the in-flight guard.
        let (tx, rx) = mpsc::channel::<StreamEvent>(SSE_BUFFER);
        let conversations = self.conversations.clone();
        let memory = self.memory.clone();
        let provider_label = agent.provider.clone();
        tokio::spawn(stream_task(
            guard,
            tx,
            meta,
            delta_stream,
            conversations,
            memory,
            conversation_id,
            assistant.id,
            provider_label,
        ));

        Ok(rx)
    }

    fn acquire(&self, id: Uuid) -> AppResult<InflightGuard> {
        let mut set = self
            .inflight
            .lock()
            .map_err(|_| AppError::Internal("inflight lock poisoned".into()))?;
        if set.contains(&id) {
            return Err(AppError::Conflict(
                "a response is already streaming for this conversation".into(),
            ));
        }
        set.insert(id);
        Ok(InflightGuard { set: self.inflight.clone(), id })
    }

    /// Insert a new user message, or (on retry) reuse the trailing user message
    /// after discarding a prior errored/stopped assistant reply. Returns the
    /// query text used for memory + dispatch.
    async fn resolve_user_message(
        &self,
        conversation_id: Uuid,
        req: &ChatStartRequest,
    ) -> AppResult<(String, Uuid)> {
        if req.retry {
            // Drop a trailing assistant turn so the user message is last again.
            if let Some(last) = self.conversations.last_message(conversation_id).await? {
                if last.role == "assistant"
                    && (last.status == "error" || last.status == "stopped")
                {
                    self.conversations.delete_message(last.id).await?;
                }
            }
            match self.conversations.last_message(conversation_id).await? {
                Some(m) if m.role == "user" => Ok((m.content, m.id)),
                _ => Err(AppError::validation(
                    "nothing to retry: no trailing user message",
                )),
            }
        } else {
            let content = req
                .content
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .ok_or_else(|| AppError::validation_field("content", "message cannot be empty"))?;
            if content.chars().count() > 20_000 {
                return Err(AppError::validation_field(
                    "content",
                    "message must be at most 20000 characters",
                ));
            }
            let msg = self
                .conversations
                .insert_message(conversation_id, NewMessage::user(content))
                .await?;
            Ok((msg.content, msg.id))
        }
    }

    async fn resolve_key(&self, agent: &crate::agents::model::Agent) -> Option<String> {
        if !agent.has_override_key {
            return None;
        }
        self.secrets
            .get(&key_slug(agent.id, &agent.provider))
            .await
            .ok()
    }

    async fn resolve_base_url(&self, provider: &str) -> Option<String> {
        if provider != "openai_compat" {
            return None;
        }
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT base_url FROM provider_keys WHERE name = $1",
        )
        .bind(provider)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .flatten()
    }
}

#[allow(clippy::too_many_arguments)]
async fn stream_task(
    guard: InflightGuard,
    mut tx: mpsc::Sender<StreamEvent>,
    meta: StreamEvent,
    mut delta_stream: crate::llm::ChatStream,
    conversations: ConversationsService,
    memory: MemoryService,
    conversation_id: Uuid,
    assistant_message_id: Uuid,
    provider_label: String,
) {
    let _guard = guard; // released when this task ends

    // Meta first so the client can render recalled turns before tokens arrive.
    let mut client_gone = tx.send(meta).await.is_err();

    let mut acc = String::new();
    let mut usage = ChatUsage::default();
    let mut finish: Option<String> = None;
    let mut errored: Option<(String, String, Option<String>)> = None;

    if !client_gone {
        while let Some(item) = delta_stream.next().await {
            match item {
                Ok(delta) => {
                    if let Some(u) = delta.usage {
                        if u.input_tokens > 0 {
                            usage.input_tokens = u.input_tokens;
                        }
                        if u.output_tokens > 0 {
                            usage.output_tokens = u.output_tokens;
                        }
                    }
                    if let Some(fr) = delta.finish_reason {
                        finish = Some(fr);
                    }
                    if !delta.content.is_empty() {
                        acc.push_str(&delta.content);
                        if tx
                            .send(StreamEvent::Chunk { content: delta.content })
                            .await
                            .is_err()
                        {
                            client_gone = true;
                            break;
                        }
                    }
                }
                Err(e) => {
                    errored = Some(chat_error_parts(e, &provider_label));
                    break;
                }
            }
        }
    }

    let status = if errored.is_some() {
        "error"
    } else if client_gone {
        "stopped"
    } else {
        "complete"
    };
    let token_count = if usage.output_tokens > 0 {
        usage.output_tokens as i32
    } else {
        (acc.chars().count() as i64 / CHARS_PER_TOKEN) as i32
    };

    let _ = conversations
        .update_message(assistant_message_id, &acc, status, Some(token_count), finish.as_deref())
        .await;

    match errored {
        Some((code, message, provider)) => {
            let _ = tx.send(StreamEvent::Error { code, message, provider }).await;
        }
        None if !client_gone => {
            let _ = tx
                .send(StreamEvent::Done {
                    status: status.to_string(),
                    token_count,
                    finish_reason: finish,
                })
                .await;
        }
        None => {}
    }

    // Embed older turns after the reply settles (skip on hard error).
    if status != "error" {
        let _ = memory.embed_pending(conversation_id).await;
    }
}

/// Splits a mid-stream `ChatError` into SSE error fields.
fn chat_error_parts(e: ChatError, provider: &str) -> (String, String, Option<String>) {
    match e {
        ChatError::NoKey(p) => (
            "provider_error".into(),
            format!("no API key configured for {p}"),
            Some(provider.to_string()),
        ),
        ChatError::Http { status, body } => (
            "provider_error".into(),
            format!("provider returned {status}: {body}"),
            Some(provider.to_string()),
        ),
        ChatError::Network(m) | ChatError::Backend(m) | ChatError::Stream(m) => {
            ("provider_error".into(), m, Some(provider.to_string()))
        }
    }
}

/// Maps a preflight `ChatError` to an `AppError` with the provider's status.
fn chat_error_to_app(e: ChatError, provider: &str) -> AppError {
    match e {
        ChatError::NoKey(p) => AppError::Provider {
            provider: provider.to_string(),
            code: "provider_error".into(),
            message: format!("no API key configured for {p}"),
            status: StatusCode::UNAUTHORIZED,
        },
        ChatError::Http { status, body } => AppError::Provider {
            provider: provider.to_string(),
            code: "provider_error".into(),
            message: body,
            status: StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY),
        },
        ChatError::Network(m) | ChatError::Backend(m) | ChatError::Stream(m) => {
            AppError::Provider {
                provider: provider.to_string(),
                code: "provider_error".into(),
                message: m,
                status: StatusCode::BAD_GATEWAY,
            }
        }
    }
}

/// Resolve the effective response language (F09): a specific per-agent
/// `response_language` wins; otherwise the request's UI locale; otherwise
/// English. `auto` and unsupported request locales fall through to `en`.
fn resolve_response_language(agent_language: &str, req_locale: Option<&str>) -> String {
    if crate::i18n::is_supported(agent_language) {
        return agent_language.to_string();
    }
    match req_locale {
        Some(l) if crate::i18n::is_supported(l) => l.to_string(),
        _ => "en".to_string(),
    }
}

/// Prepend a "respond in <language>" directive to the composed system prompt
/// unless the resolved language is English (the model's default).
fn apply_language_directive(agent_language: &str, req_locale: Option<&str>, base: &str) -> String {
    let lang = resolve_response_language(agent_language, req_locale);
    if lang == "en" {
        return base.to_string();
    }
    let name = crate::i18n::language_name(&lang);
    format!("Respond in {name}.\n\n{base}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_override_wins_over_request_locale() {
        assert_eq!(resolve_response_language("pt-BR", Some("en")), "pt-BR");
    }

    #[test]
    fn auto_falls_through_to_request_locale() {
        assert_eq!(resolve_response_language("auto", Some("pt-BR")), "pt-BR");
    }

    #[test]
    fn auto_without_locale_defaults_to_en() {
        assert_eq!(resolve_response_language("auto", None), "en");
        assert_eq!(resolve_response_language("auto", Some("fr")), "en");
    }

    #[test]
    fn directive_added_for_portuguese() {
        let out = apply_language_directive("auto", Some("pt-BR"), "BASE");
        assert!(out.starts_with("Respond in Brazilian Portuguese."));
        assert!(out.ends_with("BASE"));
    }

    #[test]
    fn no_directive_for_english() {
        let out = apply_language_directive("auto", Some("en"), "BASE");
        assert_eq!(out, "BASE");
    }
}
