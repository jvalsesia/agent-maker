import type { Locale } from "@/i18n/locales/supported";

export type ProviderName = "anthropic" | "openai" | "openai_compat";

/** `auto` follows the active UI locale; a specific locale forces that language. */
export type ResponseLanguage = "auto" | Locale;

export interface SettingsDto {
  default_provider: ProviderName;
  default_model: Record<ProviderName, string | null>;
  memory_defaults: { recent_n: number; top_k: number };
  appearance: { theme: "light" | "dark" | "system"; locale: Locale };
  providers: ProviderEntry[];
  key_store_backend: "keyring" | "file";
}

export interface ProviderEntry {
  name: ProviderName;
  key_configured: boolean;
  key_masked: string | null;
  base_url: string | null;
}

export interface ApiErrorBody {
  error: { code: string; message: string; provider?: string; field?: string };
}

export class ApiError extends Error {
  constructor(public status: number, public body: ApiErrorBody) {
    super(body.error?.message ?? `HTTP ${status}`);
  }
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: body ? { "content-type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (res.status === 204) return undefined as T;
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new ApiError(res.status, data as ApiErrorBody);
  return data as T;
}

// ----- Agents (F02) -----

export interface AgentSummary {
  id: string;
  name: string;
  preamble: string | null;
  provider: ProviderName;
  model: string;
  has_override_key: boolean;
  last_used_at: string | null;
  created_at: string;
  attached_skill_count: number;
  conversation_count: number;
}

export interface Agent {
  id: string;
  name: string;
  preamble: string | null;
  system_prompt: string;
  provider: ProviderName;
  model: string;
  has_override_key: boolean;
  recent_n_override: number | null;
  top_k_override: number | null;
  response_language: ResponseLanguage;
  created_at: string;
  updated_at: string;
  last_used_at: string | null;
}

export interface AgentWarning {
  field: string;
  message: string;
}

export interface AgentResponse {
  agent: Agent;
  warnings: AgentWarning[];
}

export interface AgentUpsert {
  name: string;
  preamble?: string | null;
  system_prompt: string;
  provider: ProviderName;
  model: string;
  recent_n_override?: number | null;
  top_k_override?: number | null;
  response_language?: ResponseLanguage;
}

export type AgentSort = "name" | "last_used" | "created";
export type AgentOrder = "asc" | "desc";

export interface ListAgentsParams {
  sort?: AgentSort;
  order?: AgentOrder;
  q?: string;
}

function qs(params: Record<string, string | undefined>): string {
  const entries = Object.entries(params).filter(([, v]) => v !== undefined && v !== "");
  if (entries.length === 0) return "";
  return "?" + entries.map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v as string)}`).join("&");
}

export const api = {
  getSettings: () => request<SettingsDto>("GET", "/api/settings"),
  putSettings: (patch: Partial<{
    default_provider: ProviderName;
    default_model: Partial<Record<ProviderName, string>>;
    memory_defaults: Partial<{ recent_n: number; top_k: number }>;
    appearance: Partial<{ theme: "light" | "dark" | "system"; locale: Locale }>;
  }>) => request<SettingsDto>("PUT", "/api/settings", patch),
  putKey: (name: ProviderName, key: string, base_url?: string | null) =>
    request<void>("PUT", `/api/settings/providers/${name}/key`, { key, base_url }),
  deleteKey: (name: ProviderName) =>
    request<void>("DELETE", `/api/settings/providers/${name}/key`),
  testProvider: (name: ProviderName) =>
    request<{ ok: true; model_used: string; latency_ms: number }>(
      "POST",
      `/api/settings/providers/${name}/test`,
    ),
  wipe: () => request<void>("POST", "/api/settings/data/wipe", { confirm: "WIPE" }),
  health: () => request<{ status: string; db: string; key_store: string }>("GET", "/api/health"),

  listAgents: (p: ListAgentsParams = {}) =>
    request<{ agents: AgentSummary[] }>(
      "GET",
      `/api/agents${qs({ sort: p.sort, order: p.order, q: p.q })}`,
    ),
  getAgent: (id: string) => request<{ agent: Agent }>("GET", `/api/agents/${id}`),
  createAgent: (body: AgentUpsert) => request<AgentResponse>("POST", "/api/agents", body),
  updateAgent: (id: string, body: AgentUpsert) =>
    request<AgentResponse>("PUT", `/api/agents/${id}`, body),
  deleteAgent: (id: string) => request<void>("DELETE", `/api/agents/${id}`),
  cloneAgent: (id: string, name?: string) =>
    request<AgentResponse>("POST", `/api/agents/${id}/clone`, name ? { name } : {}),
  saveAgentKey: (id: string, key: string) =>
    request<{ has_override_key: true; key_masked: string }>(
      "PUT",
      `/api/agents/${id}/key`,
      { key },
    ),
  deleteAgentKey: (id: string) => request<void>("DELETE", `/api/agents/${id}/key`),
  getAgentModels: (provider: ProviderName) =>
    request<{ provider: ProviderName; models: string[] }>(
      "GET",
      `/api/agents/models?provider=${provider}`,
    ),

  // ----- Skills (F03) -----
  listSkills: (p: ListSkillsParams = {}) =>
    request<{ skills: SkillSummary[] }>(
      "GET",
      `/api/skills${qs({ sort: p.sort, order: p.order, q: p.q })}`,
    ),
  getSkill: (id: string) =>
    request<{ skill: Skill; using_agents: UsingAgent[] }>("GET", `/api/skills/${id}`),
  createSkill: (body: SkillUpsert) =>
    request<SkillResponse>("POST", "/api/skills", body),
  updateSkill: (id: string, body: SkillUpsert) =>
    request<SkillResponse>("PUT", `/api/skills/${id}`, body),
  deleteSkill: (id: string) => request<void>("DELETE", `/api/skills/${id}`),
  cloneSkill: (id: string, name?: string) =>
    request<SkillResponse>("POST", `/api/skills/${id}/clone`, name ? { name } : {}),

  // ----- Skill attachments (F04) -----
  listAttachedSkills: (agentId: string) =>
    request<{ attached: AttachedSkill[] }>("GET", `/api/agents/${agentId}/skills`),
  replaceAttachedSkills: (agentId: string, skill_ids: string[]) =>
    request<AttachmentsResponse>("PUT", `/api/agents/${agentId}/skills`, { skill_ids }),
  detachSkill: (agentId: string, skillId: string) =>
    request<void>("DELETE", `/api/agents/${agentId}/skills/${skillId}`),
  getComposePreview: (agentId: string) =>
    request<ComposePreview>("GET", `/api/agents/${agentId}/compose`),

  // ----- Conversations (F06) -----
  listConversations: (agentId: string) =>
    request<{ conversations: Conversation[] }>(
      "GET",
      `/api/agents/${agentId}/conversations`,
    ),
  createConversation: (agentId: string, title?: string) =>
    request<{ conversation: Conversation; warnings: { field: string; message: string }[] }>(
      "POST",
      `/api/agents/${agentId}/conversations`,
      title ? { title } : {},
    ),
  renameConversation: (id: string, title: string) =>
    request<{ conversation: Conversation; warnings: { field: string; message: string }[] }>(
      "PATCH",
      `/api/conversations/${id}`,
      { title },
    ),
  deleteConversation: (id: string) =>
    request<void>("DELETE", `/api/conversations/${id}`),
  listMessages: (id: string) =>
    request<{ conversation: Conversation; messages: Message[] }>(
      "GET",
      `/api/conversations/${id}/messages`,
    ),

  // ----- Memory (F08) -----
  queryMemory: (conversationId: string, body: MemoryQueryInput) =>
    request<MemoryBlock>(
      "POST",
      `/api/conversations/${conversationId}/memory/query`,
      body,
    ),
  embedPending: (conversationId: string) =>
    request<EmbedReport>(
      "POST",
      `/api/conversations/${conversationId}/memory/embed-pending`,
      {},
    ),
  clearMemory: (conversationId: string) =>
    request<ClearReport>(
      "DELETE",
      `/api/conversations/${conversationId}/memory`,
    ),
  memoryStats: (conversationId: string) =>
    request<{ embedded: number }>(
      "GET",
      `/api/conversations/${conversationId}/memory`,
    ),
};

// ----- Memory types (F08) -----

export interface MemoryQueryInput {
  query: string;
  recent_n?: number;
  top_k?: number;
}

export interface MemoryTurn {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
}

export interface RetrievedTurn extends MemoryTurn {
  similarity: number;
}

export interface MemoryBlock {
  conversation_id: string;
  agent_id: string;
  effective_n: number;
  effective_k: number;
  recent: MemoryTurn[];
  retrieved: RetrievedTurn[];
  degraded: boolean;
  degraded_reason?: string | null;
}

export interface EmbedReport {
  embedded: number;
  skipped_already_present: number;
  failed: number;
}

export interface ClearReport {
  removed: number;
}

// ----- Conversation types (F06) -----

export interface Conversation {
  id: string;
  agent_id: string;
  title: string;
  created_at: string;
  last_activity_at: string;
  message_count: number;
}

export interface RecalledRef {
  message_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
  similarity: number;
}

export interface Message {
  id: string;
  conversation_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  status: "complete" | "stopped" | "error";
  model: string | null;
  token_count: number | null;
  finish_reason?: string | null;
  created_at: string;
  recalled?: RecalledRef[];
}

// ----- Chat runtime types (F07) -----

export interface ChatRecalledTurn {
  message_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  created_at: string;
  similarity: number;
}

export type ChatStreamEvent =
  | {
      type: "meta";
      user_message_id: string;
      assistant_message_id: string;
      model: string;
      degraded: boolean;
      degraded_reason?: string | null;
      recalled: ChatRecalledTurn[];
    }
  | { type: "chunk"; content: string }
  | { type: "done"; status: "complete" | "stopped"; token_count: number; finish_reason?: string | null }
  | { type: "error"; code: string; message: string; provider?: string };

export interface ChatStartInput {
  content?: string;
  retry?: boolean;
  recent_n?: number;
  top_k?: number;
  /** Active UI locale; used when the agent's response_language is `auto`. */
  locale?: Locale;
}

/**
 * Stream a chat turn. POSTs to the SSE endpoint and yields parsed events. Pass
 * an `AbortSignal` to stop the stream (the backend persists the partial reply
 * as "stopped"). Pre-stream failures throw `ApiError`.
 */
export async function* chatStream(
  conversationId: string,
  input: ChatStartInput,
  signal?: AbortSignal,
): AsyncGenerator<ChatStreamEvent> {
  const res = await fetch(`/api/conversations/${conversationId}/chat`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
    signal,
  });
  if (!res.ok || !res.body) {
    const data = await res.json().catch(() => ({}));
    throw new ApiError(res.status, data as ApiErrorBody);
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let nl: number;
      while ((nl = buffer.indexOf("\n")) >= 0) {
        const line = buffer.slice(0, nl).trimEnd();
        buffer = buffer.slice(nl + 1);
        if (line.startsWith("data:")) {
          const payload = line.slice(5).trim();
          if (payload) yield JSON.parse(payload) as ChatStreamEvent;
        }
      }
    }
  } finally {
    reader.cancel().catch(() => {});
  }
}

// ----- Skill attachment types (F04) -----

export interface AttachedSkill {
  skill_id: string;
  name: string;
  description: string;
  position: number;
}

export interface AttachmentWarning {
  field: string;
  message: string;
}

export interface AttachmentsResponse {
  attached: AttachedSkill[];
  warnings: AttachmentWarning[];
}

export interface ComposePreview {
  composed: string;
  length_chars: number;
  model_context_chars: number;
  fraction: number;
  warning: string | null;
}

// ----- Skills types (F03) -----

export interface Skill {
  id: string;
  name: string;
  description: string;
  body: string;
  created_at: string;
  updated_at: string;
}

export interface SkillSummary {
  id: string;
  name: string;
  description: string;
  created_at: string;
  updated_at: string;
  attached_agent_count: number;
}

export interface UsingAgent {
  id: string;
  name: string;
}

export interface SkillWarning {
  field: string;
  message: string;
}

export interface SkillResponse {
  skill: Skill;
  warnings: SkillWarning[];
}

export interface SkillUpsert {
  name: string;
  description: string;
  body: string;
}

export type SkillSort = "name" | "attached" | "created";

export interface ListSkillsParams {
  sort?: SkillSort;
  order?: AgentOrder;
  q?: string;
}
