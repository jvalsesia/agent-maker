use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use sqlx::PgPool;
use uuid::Uuid;

use super::compose::{SubagentReply, append_subagent_responses, compose};
use super::model::{ChatStartRequest, RecalledTurn, StreamEvent, SubagentNotice};
use crate::agents::AgentsService;
use crate::agents::model::Agent;
use crate::agents::service::key_slug;
use crate::conversations::ConversationsService;
use crate::conversations::model::{Message, NewMessage};
use crate::error::{AppError, AppResult};
use crate::llm::{
    ChatError, ChatMessage, ChatRequest, ChatRole, ChatStream, ChatUsage, ProviderName,
    ProviderRegistry,
};
use crate::memory::MemoryService;
use crate::memory::model::{MemoryBlock, MemoryQuery};
use crate::secrets::{AnyStore, SecretStore};
use crate::skill_attachments::AttachmentsService;
use crate::skill_attachments::service::model_context_chars;
use crate::subagents::SubagentsService;
use crate::subagents::model::AttachedSubagent;

/// Default ceiling on generated tokens per turn, and the per-token char reserve
/// subtracted from the model budget to leave room for the response.
const DEFAULT_MAX_TOKENS: u32 = 4096;
const CHARS_PER_TOKEN: i64 = 4;
const SSE_BUFFER: usize = 64;

/// Maximum distinct sub-agent mentions honored per turn (F11). Extra distinct
/// matched aliases are ignored and surfaced as an `overflow` notice.
const MAX_MENTIONS: usize = 3;

/// Runtime delegation depth guardrail. A delegated child does not itself parse
/// `@mentions` in v1, so the effective depth is 1; this constant documents the
/// ceiling the design assumes.
#[allow(dead_code)]
const MAX_DELEGATION_DEPTH: u8 = 2;

#[derive(Clone)]
pub struct ChatService {
    pub pool: PgPool,
    pub providers: ProviderRegistry,
    pub agents: AgentsService,
    pub conversations: ConversationsService,
    pub memory: MemoryService,
    pub attachments: AttachmentsService,
    pub subagents: SubagentsService,
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
        subagents: SubagentsService,
        secrets: Arc<AnyStore>,
    ) -> Self {
        Self {
            pool,
            providers,
            agents,
            conversations,
            memory,
            attachments,
            subagents,
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

        // F11: route the turn through any `@mentioned` sub-agents attached to
        // this agent. `matched` is de-duplicated, mention-ordered, capped at 3.
        let attached = self.subagents.list(agent_id).await.unwrap_or_default();
        let parsed = parse_mentions(&user_query, &attached);
        let notices = build_notices(&parsed);

        let (tx, rx) = mpsc::channel::<StreamEvent>(SSE_BUFFER);

        if parsed.matched.is_empty() {
            // ---- No delegation: open the parent stream now so provider
            // preflight (4xx/auth) surfaces as an HTTP error before any token. ----
            let composed = compose(&base_system, &block, budget, reserve)?;
            let api_key = self.resolve_key(&agent).await;
            let base_url = self.resolve_base_url(&agent.provider).await;

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

            let assistant = self
                .persist_assistant_placeholder(conversation_id, agent_id, &agent.model, &block)
                .await?;
            let meta = build_meta(user_message_id, assistant.id, &agent.model, &block, notices);

            tokio::spawn(stream_task(
                guard,
                tx,
                meta,
                delta_stream,
                self.conversations.clone(),
                self.memory.clone(),
                conversation_id,
                assistant.id,
                agent.provider.clone(),
            ));
        } else {
            // ---- Delegation: the children run inside the spawned task; the
            // parent stream (and its preflight errors) lives on the SSE path so
            // each labeled sub-agent bubble lands as soon as it is produced. ----
            let assistant = self
                .persist_assistant_placeholder(conversation_id, agent_id, &agent.model, &block)
                .await?;
            let meta = build_meta(user_message_id, assistant.id, &agent.model, &block, notices);

            let svc = self.clone();
            tokio::spawn(svc.run_delegation(DelegationContext {
                guard,
                tx,
                meta,
                conversation_id,
                assistant_message_id: assistant.id,
                agent,
                matched: parsed.matched,
                child_task: parsed.task,
                locale: req.locale.clone(),
                base_system,
                block,
                budget,
                reserve,
            }));
        }

        Ok(rx)
    }

    /// Insert the empty assistant turn, attach recalled-turn references, and
    /// touch the agent's last-used timestamp. Shared by both chat paths.
    async fn persist_assistant_placeholder(
        &self,
        conversation_id: Uuid,
        agent_id: Uuid,
        model: &str,
        block: &MemoryBlock,
    ) -> AppResult<Message> {
        let assistant = self
            .conversations
            .insert_message(conversation_id, NewMessage::assistant("", "complete", Some(model)))
            .await?;
        let recall_pairs: Vec<(Uuid, f32)> =
            block.retrieved.iter().map(|r| (r.id, r.similarity)).collect();
        self.conversations
            .insert_recalls(assistant.id, &recall_pairs)
            .await?;
        let _ = sqlx::query("UPDATE agents SET last_used_at = now() WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await;
        Ok(assistant)
    }

    /// Run one sub-agent to completion: compose the child's own system prompt
    /// (its skills + F09 directive), dispatch a single-user-turn request through
    /// the child's own provider/model/key, and drain the stream to a `String`.
    /// The child is stateless — it sees only `task`, never the parent history.
    async fn run_subagent_blocking(
        &self,
        child_id: Uuid,
        task: &str,
        locale: Option<&str>,
    ) -> AppResult<ChildResult> {
        let child = self.agents.get(child_id).await?;
        let provider_name = ProviderName::from_str(&child.provider)
            .map_err(|e| AppError::Internal(format!("bad provider on sub-agent: {e}")))?;
        let composed_base = self.attachments.compose(child_id).await?.composed;
        let system = apply_language_directive(&child.response_language, locale, &composed_base);
        let api_key = self.resolve_key(&child).await;
        let base_url = self.resolve_base_url(&child.provider).await;
        let model = child.model.clone();

        let chat_req = ChatRequest {
            model: child.model.clone(),
            system: Some(system),
            messages: vec![ChatMessage { role: ChatRole::User, content: task.to_string() }],
            base_url,
            api_key,
            max_tokens: DEFAULT_MAX_TOKENS,
        };

        let provider = self.providers.get(provider_name);
        let mut stream = match provider.chat(chat_req).await {
            Ok(s) => s,
            Err(e) => {
                let (_, message, _) = chat_error_parts(e, &child.provider);
                return Ok(ChildResult { content: Err(message), model });
            }
        };

        let mut acc = String::new();
        while let Some(item) = stream.next().await {
            match item {
                Ok(delta) => acc.push_str(&delta.content),
                Err(e) => {
                    let (_, message, _) = chat_error_parts(e, &child.provider);
                    return Ok(ChildResult { content: Err(message), model });
                }
            }
        }
        Ok(ChildResult { content: Ok(acc), model })
    }

    /// The delegation path's spawned task: drain each child, persist + emit its
    /// turn, hold the parent on any child error, otherwise stream the parent
    /// reply composed with the "Sub-agent responses" block.
    async fn run_delegation(self, ctx: DelegationContext) {
        let DelegationContext {
            guard,
            mut tx,
            meta,
            conversation_id,
            assistant_message_id,
            agent,
            matched,
            child_task,
            locale,
            base_system,
            block,
            budget,
            reserve,
        } = ctx;
        let _guard = guard;

        if tx.send(meta).await.is_err() {
            // Client hung up before any work; settle the placeholder as stopped.
            let _ = self
                .conversations
                .update_message(assistant_message_id, "", "stopped", Some(0), None)
                .await;
            return;
        }

        let mut replies: Vec<SubagentReply> = Vec::new();
        for sub in &matched {
            let outcome = self
                .run_subagent_blocking(sub.child_id, &child_task, locale.as_deref())
                .await;
            let result = match outcome {
                Ok(r) => r,
                Err(e) => ChildResult { content: Err(e.to_string()), model: String::new() },
            };

            match result.content {
                Ok(content) => {
                    let model = (!result.model.is_empty()).then_some(result.model.as_str());
                    let persisted = self
                        .conversations
                        .insert_message(
                            conversation_id,
                            NewMessage::subagent(&content, "complete", model, &sub.alias, sub.child_id),
                        )
                        .await;
                    let message_id = persisted.map(|m| m.id).unwrap_or_else(|_| Uuid::nil());
                    let _ = tx
                        .send(StreamEvent::Subagent {
                            message_id,
                            alias: sub.alias.clone(),
                            agent_id: sub.child_id,
                            agent_name: sub.name.clone(),
                            content: content.clone(),
                            status: "complete".into(),
                            error: None,
                        })
                        .await;
                    replies.push(SubagentReply {
                        alias: sub.alias.clone(),
                        name: sub.name.clone(),
                        content,
                    });
                }
                Err(message) => {
                    // A failed sub-agent holds the parent: persist the error turn,
                    // emit it + an Error frame, and stop without synthesizing.
                    let model = (!result.model.is_empty()).then_some(result.model.as_str());
                    let persisted = self
                        .conversations
                        .insert_message(
                            conversation_id,
                            NewMessage::subagent("", "error", model, &sub.alias, sub.child_id),
                        )
                        .await;
                    let message_id = persisted.map(|m| m.id).unwrap_or_else(|_| Uuid::nil());
                    let _ = tx
                        .send(StreamEvent::Subagent {
                            message_id,
                            alias: sub.alias.clone(),
                            agent_id: sub.child_id,
                            agent_name: sub.name.clone(),
                            content: String::new(),
                            status: "error".into(),
                            error: Some(message.clone()),
                        })
                        .await;
                    let _ = self
                        .conversations
                        .update_message(assistant_message_id, "", "error", Some(0), None)
                        .await;
                    let _ = tx
                        .send(StreamEvent::Error {
                            code: "provider_error".into(),
                            message,
                            provider: Some(agent.provider.clone()),
                        })
                        .await;
                    return;
                }
            }
        }

        // All children succeeded → compose the parent prompt with their replies.
        let augmented = append_subagent_responses(&base_system, &replies);
        let composed = match compose(&augmented, &block, budget, reserve) {
            Ok(c) => c,
            Err(e) => {
                let _ = self
                    .conversations
                    .update_message(assistant_message_id, "", "error", Some(0), None)
                    .await;
                let _ = tx
                    .send(StreamEvent::Error {
                        code: "context_too_large".into(),
                        message: e.to_string(),
                        provider: None,
                    })
                    .await;
                return;
            }
        };

        let provider_name = match ProviderName::from_str(&agent.provider) {
            Ok(p) => p,
            Err(e) => {
                let _ = tx
                    .send(StreamEvent::Error {
                        code: "internal_error".into(),
                        message: format!("bad provider on agent: {e}"),
                        provider: None,
                    })
                    .await;
                return;
            }
        };
        let api_key = self.resolve_key(&agent).await;
        let base_url = self.resolve_base_url(&agent.provider).await;
        let chat_req = ChatRequest {
            model: agent.model.clone(),
            system: Some(composed.system),
            messages: composed.messages,
            base_url,
            api_key,
            max_tokens: DEFAULT_MAX_TOKENS,
        };
        let delta_stream = match self.providers.get(provider_name).chat(chat_req).await {
            Ok(s) => s,
            Err(e) => {
                let (code, message, provider) = chat_error_parts(e, &agent.provider);
                let _ = self
                    .conversations
                    .update_message(assistant_message_id, "", "error", Some(0), None)
                    .await;
                let _ = tx.send(StreamEvent::Error { code, message, provider }).await;
                return;
            }
        };

        pump_stream(
            tx,
            delta_stream,
            self.conversations.clone(),
            self.memory.clone(),
            conversation_id,
            assistant_message_id,
            agent.provider.clone(),
            false,
        )
        .await;
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
            if let Some(last) = self.conversations.last_message(conversation_id).await?
                && last.role == "assistant"
                    && (last.status == "error" || last.status == "stopped")
                {
                    self.conversations.delete_message(last.id).await?;
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
    delta_stream: crate::llm::ChatStream,
    conversations: ConversationsService,
    memory: MemoryService,
    conversation_id: Uuid,
    assistant_message_id: Uuid,
    provider_label: String,
) {
    let _guard = guard; // released when this task ends

    // Meta first so the client can render recalled turns before tokens arrive.
    let client_gone = tx.send(meta).await.is_err();
    pump_stream(
        tx,
        delta_stream,
        conversations,
        memory,
        conversation_id,
        assistant_message_id,
        provider_label,
        client_gone,
    )
    .await;
}

/// Drain a provider stream into the persisted assistant turn, forwarding chunks
/// as SSE and settling the message + memory at the end. `client_gone` short-
/// circuits the loop when the SSE receiver has already dropped. Shared by the
/// direct path (`stream_task`) and the delegation path (`run_delegation`).
#[allow(clippy::too_many_arguments)]
async fn pump_stream(
    mut tx: mpsc::Sender<StreamEvent>,
    mut delta_stream: ChatStream,
    conversations: ConversationsService,
    memory: MemoryService,
    conversation_id: Uuid,
    assistant_message_id: Uuid,
    provider_label: String,
    mut client_gone: bool,
) {
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

/// Result of resolving `@mentions` in a user message against attached sub-agents.
struct ParsedMentions {
    /// Honored mentions: de-duplicated, in first-occurrence order, capped at 3.
    matched: Vec<AttachedSubagent>,
    /// The text handed to the children: matched handles stripped, unmatched left
    /// as literal text, whitespace collapsed.
    task: String,
    /// Matched aliases ignored because the per-turn cap was already reached.
    overflow: Vec<String>,
    /// `@handles` that matched no attached sub-agent (kept literal in `task`).
    unknown: Vec<String>,
}

/// One drained sub-agent call: either its full reply or a provider error message,
/// plus the model the child used.
struct ChildResult {
    content: Result<String, String>,
    model: String,
}

/// Everything the spawned delegation task needs (bundled to keep the spawn call
/// readable).
struct DelegationContext {
    guard: InflightGuard,
    tx: mpsc::Sender<StreamEvent>,
    meta: StreamEvent,
    conversation_id: Uuid,
    assistant_message_id: Uuid,
    agent: Agent,
    matched: Vec<AttachedSubagent>,
    child_task: String,
    locale: Option<String>,
    base_system: String,
    block: MemoryBlock,
    budget: i64,
    reserve: i64,
}

/// Parse `@alias` mentions out of `text`, matching (case-insensitively) against
/// the parent's `attached` sub-agents. A mention starts at the beginning of the
/// text or after whitespace (so emails like `a@b` are ignored). Matched handles
/// are honored once each, in order, up to `MAX_MENTIONS`; the rest are reported
/// as overflow. Matched handles are stripped from the returned task; unmatched
/// ones stay literal.
fn parse_mentions(text: &str, attached: &[AttachedSubagent]) -> ParsedMentions {
    let chars: Vec<char> = text.chars().collect();
    let mut matched: Vec<AttachedSubagent> = Vec::new();
    let mut overflow: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut seen_match: HashSet<String> = HashSet::new();
    let mut seen_unknown: HashSet<String> = HashSet::new();
    let mut task = String::with_capacity(text.len());

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let at_boundary = i == 0 || chars[i - 1].is_whitespace();
        if c == '@' && at_boundary {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '-') {
                j += 1;
            }
            if j > start {
                let handle: String =
                    chars[start..j].iter().collect::<String>().to_lowercase();
                if let Some(sub) = attached.iter().find(|a| a.alias == handle) {
                    if seen_match.insert(handle.clone()) {
                        if matched.len() < MAX_MENTIONS {
                            matched.push(sub.clone());
                        } else {
                            overflow.push(handle);
                        }
                    }
                    // Strip the matched handle (and a single trailing space).
                    i = j;
                    if i < chars.len() && chars[i] == ' ' {
                        i += 1;
                    }
                    continue;
                } else if seen_unknown.insert(handle.clone()) {
                    unknown.push(handle);
                }
            }
        }
        task.push(c);
        i += 1;
    }

    let task = task.split_whitespace().collect::<Vec<_>>().join(" ");
    ParsedMentions { matched, task, overflow, unknown }
}

/// Build the `meta`-frame notices from overflow/unknown handles (empty when none).
fn build_notices(parsed: &ParsedMentions) -> Vec<SubagentNotice> {
    let mut notices = Vec::new();
    if !parsed.overflow.is_empty() {
        notices.push(SubagentNotice {
            code: "overflow".into(),
            aliases: parsed.overflow.clone(),
        });
    }
    if !parsed.unknown.is_empty() {
        notices.push(SubagentNotice {
            code: "unknown".into(),
            aliases: parsed.unknown.clone(),
        });
    }
    notices
}

/// Construct the `meta` SSE frame for a turn.
fn build_meta(
    user_message_id: Uuid,
    assistant_message_id: Uuid,
    model: &str,
    block: &MemoryBlock,
    notices: Vec<SubagentNotice>,
) -> StreamEvent {
    StreamEvent::Meta {
        user_message_id,
        assistant_message_id,
        model: model.to_string(),
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
        notices,
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

    fn sub(alias: &str) -> AttachedSubagent {
        AttachedSubagent {
            child_id: Uuid::new_v4(),
            name: alias.to_string(),
            alias: alias.to_string(),
            description: None,
            position: 0,
        }
    }

    #[test]
    fn parses_single_mention() {
        let attached = vec![sub("code-reviewer")];
        let p = parse_mentions("@code-reviewer hi", &attached);
        assert_eq!(p.matched.len(), 1);
        assert_eq!(p.matched[0].alias, "code-reviewer");
        assert_eq!(p.task, "hi");
        assert!(p.overflow.is_empty());
        assert!(p.unknown.is_empty());
    }

    #[test]
    fn mention_match_is_case_insensitive() {
        let attached = vec![sub("code-reviewer")];
        let p = parse_mentions("@Code-Reviewer please", &attached);
        assert_eq!(p.matched.len(), 1);
        assert_eq!(p.task, "please");
    }

    #[test]
    fn dedupes_and_orders() {
        let attached = vec![sub("a"), sub("b")];
        let p = parse_mentions("@a @b @a", &attached);
        let aliases: Vec<_> = p.matched.iter().map(|m| m.alias.as_str()).collect();
        assert_eq!(aliases, vec!["a", "b"]);
        assert_eq!(p.task, "");
    }

    #[test]
    fn caps_at_three() {
        let attached = vec![sub("a"), sub("b"), sub("c"), sub("d")];
        let p = parse_mentions("@a @b @c @d go", &attached);
        assert_eq!(p.matched.len(), 3);
        assert_eq!(p.overflow, vec!["d"]);
        assert_eq!(p.task, "go");
    }

    #[test]
    fn ignores_unmatched() {
        let attached = vec![sub("a")];
        let p = parse_mentions("@nope hi", &attached);
        assert!(p.matched.is_empty());
        assert_eq!(p.unknown, vec!["nope"]);
        assert_eq!(p.task, "@nope hi");
    }

    #[test]
    fn strips_only_matched() {
        let attached = vec![sub("a")];
        let p = parse_mentions("@a @nope x", &attached);
        assert_eq!(p.matched.len(), 1);
        assert_eq!(p.task, "@nope x");
        assert_eq!(p.unknown, vec!["nope"]);
    }

    #[test]
    fn ignores_email_like_at() {
        let attached = vec![sub("bar")];
        let p = parse_mentions("mail me at foo@bar.com please", &attached);
        assert!(p.matched.is_empty());
        assert_eq!(p.task, "mail me at foo@bar.com please");
    }
}
