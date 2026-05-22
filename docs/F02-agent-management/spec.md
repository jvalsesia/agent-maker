# F02. Agent Management — Technical Specification

## 1. Overview

F02 adds full CRUD plus cloning for agents — the personas that drive every chat in agent-maker. An agent is a row in the `agents` table (already created by F01) carrying a name, preamble, system prompt, provider, model, optional per-agent API key override, and optional per-agent memory overrides (`recent_n`, `top_k`). F02 implements the REST surface, the list/detail/edit UI, and the per-agent key handling. It does **not** touch skills (F04), conversations (F06), chat (F07), or memory retrieval (F08) — it only exposes the agent definitions those features consume.

## 2. Scope

**Included:**
- REST endpoints under `/api/agents/*` for list, get, create, update, delete, clone
- Per-agent key save/delete endpoints that delegate to F01's `SecretStore` under a namespaced key (`agent:<id>:provider`)
- Sort and search on the list endpoint (`?sort=name|last_used|created&order=asc|desc&q=<text>`)
- Soft validation envelope: warnings (system prompt < 50 or > 10,000 chars) returned alongside the saved entity, never blocking the save
- Hard validation: name length 1–60 and unique, system prompt required and ≤ 20,000 chars, preamble ≤ 500 chars, provider in `{anthropic, openai, openai_compat}`, model required, memory overrides bounded
- Provider/model dropdown data: a `GET /api/agents/models?provider=…` endpoint returning the static list from F01's `LlmProvider::available_models`
- Frontend pages: `/agents` (list with search + sort + "+ New"), `/agents/new`, `/agents/:id` (detail/edit), and the delete/clone modals
- "Save and chat" stub: writes the agent then navigates to `/agents/:id` with a toast — F06 will later wire the actual fresh-conversation flow
- Inline warning when the chosen provider has no configured key (cross-checks F01's provider_keys table)

**Integrated from PRD blocks:**
- `Capabilities`: field bounds, CRUD + clone, soft validation, sortable list
- `Experience`: agents list in left nav, single-page form, save-and-chat, delete confirmation w/ impact counts, one-click clone
- `Error Handling`: duplicate-name inline error, save/delete failure toasts, no-key warning with deep link

**Deferred (out of F02):**
- The actual `/agents/:id/chat/:conversationId` route (F06/F07)
- Skill picker and attachment list — placeholder section only (F04)
- The dependent-counts logic on the delete modal *for skills* uses the F01 link table; F02 surfaces conversation count from F06 only when present (graceful zero if F06 not yet shipped)

## 3. Component Overview

### Repository additions

```
/backend
├── /migrations
│   └── 0003_agents_touch.sql       # adds trigger to maintain updated_at; no schema change
└── /src
    └── agents/
        ├── mod.rs
        ├── model.rs                # Agent, AgentUpsert, AgentSummary, validation errors
        ├── service.rs              # business logic, clone, dependent counts
        └── routes.rs               # /api/agents/* handlers

/frontend
└── /src
    ├── pages/
    │   └── Agents/
    │       ├── AgentsList.tsx
    │       ├── AgentForm.tsx       # used by /new and /:id
    │       ├── DeleteAgentDialog.tsx
    │       └── index.ts
    ├── hooks/
    │   ├── useAgents.ts
    │   └── useAgent.ts
    └── lib/
        └── agents.ts               # typed REST client
```

The left-nav "Agents" item (placeholder from F01) becomes active and routes to `/agents`.

## 4. Data Model

F01's `agents` table already covers F02 in full. F02 adds **no** column changes — only a trigger to bump `updated_at` on UPDATE (migration `0003_agents_touch.sql`):

```sql
CREATE OR REPLACE FUNCTION touch_updated_at() RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER agents_touch BEFORE UPDATE ON agents
  FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
```

The same trigger is later reused by F03/F06. F02 owns its initial creation.

### Domain types

```rust
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub preamble: Option<String>,
    pub system_prompt: String,
    pub provider: Provider,            // enum reused from F01 llm module
    pub model: String,
    pub has_override_key: bool,
    pub recent_n_override: Option<i16>,
    pub top_k_override: Option<i16>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub struct AgentSummary {        // list payload
    pub id: Uuid,
    pub name: String,
    pub preamble: Option<String>,
    pub provider: Provider,
    pub model: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub attached_skill_count: i64, // 0 until F04
    pub conversation_count: i64,   // 0 until F06
}

pub struct AgentUpsert {         // create + update input
    pub name: String,
    pub preamble: Option<String>,
    pub system_prompt: String,
    pub provider: Provider,
    pub model: String,
    pub recent_n_override: Option<i16>,
    pub top_k_override: Option<i16>,
}
```

## 5. REST API

All responses use the F01 JSON error envelope. All bodies are JSON.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/agents` | List agents with search/sort |
| POST | `/api/agents` | Create agent |
| GET | `/api/agents/:id` | Get a single agent |
| PUT | `/api/agents/:id` | Update fields (full upsert) |
| DELETE | `/api/agents/:id` | Delete agent (cascades F01-defined FKs) |
| POST | `/api/agents/:id/clone` | Clone agent; body `{ "name": "optional override" }` |
| PUT | `/api/agents/:id/key` | Save per-agent API key (delegates to `SecretStore`) |
| DELETE | `/api/agents/:id/key` | Remove per-agent key override |
| GET | `/api/agents/models?provider=` | Return `available_models` for the provider |

Successful create/update returns:

```json
{
  "agent": { ...Agent... },
  "warnings": [
    { "field": "system_prompt", "message": "shorter than 50 chars" }
  ]
}
```

`warnings` is always present (possibly empty). It is informational — clients render an inline non-blocking hint.

Hard validation failures return HTTP 422 with `{ "error": { "code": "validation_failed", "details": [{ "field": …, "message": …}] } }`. Duplicate name returns 409 with `code: "duplicate_name"`.

`POST /api/agents/:id/clone` copies every column except `id`, `created_at`, `updated_at`, `last_used_at`, and `has_override_key` (clones start with the provider default key). The new name is the request body's `name` if given, else `<original> (copy)`. If the resulting name already exists, the server keeps suffixing ` (n)` (n=2,3,…) until unique.

## 6. Secrets Handling for Per-Agent Keys

Per-agent keys are stored in F01's `SecretStore` under the slug `agent:<uuid>:<provider>`. The agents table itself only holds the boolean `has_override_key` — the raw key never reaches the database. The chat runtime (F07) will resolve the key by looking up `agent:<id>:<provider>` first, then falling back to the provider default.

`PUT /api/agents/:id/key`:
- Body: `{ "key": "sk-..." }`
- Writes the secret under the namespaced slug, then sets `has_override_key = TRUE`
- Returns the masked form (last 4 chars) for display

`DELETE /api/agents/:id/key`:
- Removes the namespaced secret (if present) and sets `has_override_key = FALSE`

Agent deletion MUST also delete the namespaced secret. The service performs this in the same transaction order: secret delete first (idempotent), then DB delete; on secret-store failure the DB delete is skipped and a 500 is returned so the operator can retry.

## 7. Validation Rules

| Field | Rule | On violation |
|---|---|---|
| name | required, 1–60 chars after trim, unique | 422 `validation_failed` or 409 `duplicate_name` |
| preamble | optional, ≤ 500 chars | 422 |
| system_prompt | required, ≤ 20,000 chars | 422 |
| system_prompt | < 50 chars or > 10,000 chars | **warning** only |
| provider | required, in enum | 422 |
| model | required, non-empty | 422 |
| recent_n_override | optional, in `[4, 30]` | 422 |
| top_k_override | optional, in `[0, 10]` | 422 |

Validation is performed in `agents::service::validate` before the DB call so the failure shape is consistent across create and update.

## 8. Frontend

### Routes

- `/agents` → `AgentsList` (search input, sort dropdown, "+ New Agent" button)
- `/agents/new` → `AgentForm` in create mode
- `/agents/:id` → `AgentForm` in edit mode (loads via `useAgent`)

### Form layout (`AgentForm.tsx`)

Single page, top-to-bottom:
1. Name (Input)
2. Preamble (Input, 500-char counter)
3. System prompt (Textarea, full-width, monospace, char counter; soft-validation warnings render inline below the field as muted text)
4. Provider (Select) → Model (Select, repopulates from `/api/agents/models`)
5. Memory overrides (Inputs for N and K, both optional; placeholder shows the global default from F01 settings)
6. **Advanced** collapsible (closed by default): per-agent API key input + "Save key" / "Remove override" buttons. Shows masked form when `has_override_key` is true. Below the key input, an inline warning appears if the selected provider has no key configured globally and no per-agent override.
7. Sticky footer: "Cancel", "Save", "Save and chat", "Clone" (edit mode only), "Delete" (edit mode only, right-aligned, destructive)

Save uses TanStack Query mutations. Success → shadcn toast `Agent saved` and a brief checkmark on the button. "Save and chat" navigates to `/agents/:id` (F06 will later open a fresh conversation directly).

### List (`AgentsList.tsx`)

Search box + Sort dropdown (`Name`, `Last used`, `Created`). Each row: name, truncated preamble, provider:model badge, `last_used_at` relative time, kebab menu (`Open`, `Clone`, `Delete`). Empty state: "No agents yet — create your first one" with a primary CTA.

### Delete dialog

Modal lists: number of conversations (from `conversation_count`) and number of attached skills (from `attached_skill_count`). Buttons: `Cancel` (default focus), `Delete` (destructive, requires the count text to render even when zero). Background-click dismisses (matches F01 conventions; only F03's skill-delete is no-dismiss).

### Cross-checks with F01

- The provider/model selectors call `/api/agents/models?provider=…` (cached via TanStack Query). If F01's provider list ever returns empty for a provider, the model selector renders a manual text input as fallback.
- The `useProviderKeys` hook (added to F02 from `useSettings`) returns which providers have a saved key; used by the inline "no key configured" warning.

## 9. Error Handling

| Scenario | Backend response | Frontend behavior |
|---|---|---|
| Duplicate name | 409 `duplicate_name` | Inline field error under "Name"; form stays dirty |
| Validation failure | 422 with `details[]` | Inline field errors per `field`; form stays dirty |
| DB error on save | 500 `db_error` | Toast `Save failed — retry?`; form stays dirty |
| DB error on delete | 500 | Toast with retry; agent remains in the list (UI rollback) |
| Secret store unavailable on key save | 500 `secret_store_error` | Toast surfaces the underlying message; F01 warning behavior preserved |
| Provider with no key on selection | n/a (client-side join) | Inline amber warning under provider dropdown with a `Go to Settings` link |
| Clone produces non-unique name | server retries suffixing | Returned cloned agent already has the final unique name |

## 10. Testing Strategy

**Backend unit (`agents::service::tests`):**
- `validate` rejects every hard rule and emits warnings for soft rules
- `clone` produces correct suffix and copies all columns except excluded ones
- Per-agent key save sets `has_override_key`; delete clears it

**Backend integration (sqlx test pool against the docker-compose Postgres):**
- Full lifecycle: create → list → get → update → clone → delete; assert row counts and `updated_at` movement (trigger)
- Duplicate-name returns 409 on both create and update
- `DELETE /api/agents/:id` removes the namespaced secret first (mocked `SecretStore`)
- List sort and search return rows in the documented order

**Frontend (Vitest + Testing Library):**
- `AgentForm` renders all fields, surfaces validation errors from a mocked 422 response, and disables Save while the mutation is in-flight
- Delete dialog renders dependent counts and only fires the mutation when the destructive button is clicked
- Provider-without-key warning appears when `useProviderKeys` reports the chosen provider as missing
- `AgentsList` sort dropdown updates the query and the rendered order

**Manual smoke (documented in `verify.md`):** create → edit → clone → delete a single agent end-to-end against the running stack.

## 11. Acceptance Criteria Mapping

PRD §9 F02 → spec sections:

| Criterion | Section |
|---|---|
| Create agent with required fields, appears in list | §5 POST `/api/agents`, §8 List |
| Duplicate name returns inline error | §5, §7, §9 |
| Edit persists after reload | §5 PUT, §8 Form load via `useAgent` |
| Clone appends ` (copy)` | §5 clone |
| Delete confirmation lists conversations and skills, removes agent and its conversations | §8 Delete dialog, §4 cascades from F01 |
| Provider-without-key shows inline warning with settings link | §8 Form §9 |

Cross-feature criteria from §9 "Cross-Feature Integration" #1 (F02 agents using F01 keys in F07) and #7 (per-agent provider/model overriding defaults) are satisfied at the data-model level by `has_override_key` + the namespaced secret; F07 will consume both.
