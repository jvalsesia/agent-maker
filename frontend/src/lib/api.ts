export type ProviderName = "anthropic" | "openai" | "openai_compat";

export interface SettingsDto {
  default_provider: ProviderName;
  default_model: Record<ProviderName, string | null>;
  memory_defaults: { recent_n: number; top_k: number };
  appearance: { theme: "light" | "dark" | "system" };
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
    appearance: { theme: "light" | "dark" | "system" };
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
};

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
