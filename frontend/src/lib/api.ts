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
};
