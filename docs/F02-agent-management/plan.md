# F02. Agent Management — Implementation Plan

## Prerequisites

- F01 is merged and the stack runs locally (`docker compose up -d`, `cargo run`, `pnpm dev`)
- A valid provider key configured in Settings (so the per-agent override path can be exercised)
- F01's `SecretStore`, `LlmProvider::available_models`, and `agents` table schema are already in place

## Phase 1 — Backend Data and Service Layer

1. **Migration `0003_agents_touch.sql`** — Add the `touch_updated_at()` plpgsql function and the `agents_touch` BEFORE UPDATE trigger. No column changes. Confirm the migration runs cleanly against the existing F01 database.

2. **`agents::model` module** — Define `Agent`, `AgentSummary`, `AgentUpsert`, and the `ValidationError` / `Warning` types. Derive `serde::Serialize/Deserialize` and `sqlx::FromRow`. Reuse the `Provider` enum from F01's `llm` module.

3. **`agents::service::validate`** — Centralize the hard + soft validation rules from spec §7. Return `Result<Vec<Warning>, Vec<ValidationError>>` so create and update share the same code path.

4. **`agents::service` CRUD** — Implement `list(filter)`, `get(id)`, `create(upsert)`, `update(id, upsert)`, `delete(id, secret_store)`, and `clone(id, override_name)`. `list` joins to `agent_skills` and `conversations` for the count columns using `LEFT JOIN ... GROUP BY` (counts default to 0). Delete must call `secret_store.remove("agent:<id>:<provider>")` before the SQL DELETE; on secret-store failure, bubble up a 500 and leave the row intact.

## Phase 2 — Backend REST Surface

5. **`agents::routes` handlers** — Mount the nine endpoints from spec §5 under `/api/agents`. Use Axum extractors for path/query/body. Map `ValidationError` → 422, `duplicate_name` (Postgres `23505`) → 409, secret-store errors → 500 with the F01 envelope.

6. **Per-agent key endpoints** — `PUT /api/agents/:id/key` writes the secret under `agent:<id>:<provider>` and flips `has_override_key`. `DELETE` does the inverse. Return the masked form (last 4 chars + `••••`) on PUT.

7. **`GET /api/agents/models` endpoint** — Thin wrapper around `LlmProvider::available_models(provider)`. Cache the result in memory (process-lifetime) since the list is static per provider in F01.

8. **Wire routes into the Axum router** — Add `agents::routes::router()` under the existing `/api` `Router::nest` call in `main.rs`.

## Phase 3 — Frontend

9. **REST client (`lib/agents.ts`)** — Strongly typed wrappers (`listAgents`, `getAgent`, `createAgent`, `updateAgent`, `deleteAgent`, `cloneAgent`, `saveAgentKey`, `deleteAgentKey`, `getModels`). All functions throw a typed `ApiError` matching the F01 envelope so React components can branch on `code`.

10. **React Query hooks (`useAgents`, `useAgent`, `useAgentModels`, `useProviderKeys`)** — Standard query/mutation pairs with invalidation: a successful `createAgent`/`updateAgent`/`deleteAgent`/`cloneAgent` invalidates `['agents']`; key save invalidates `['agent', id]`.

11. **Pages and components** — Build `AgentsList`, `AgentForm` (shared by `/agents/new` and `/agents/:id`), and `DeleteAgentDialog` per spec §8. Use shadcn primitives already present from F01 (`button`, `input`, `label`, `textarea` — add if missing —, `select`, `dialog`, `toast`).

12. **Routing and navigation** — Add `/agents`, `/agents/new`, `/agents/:id` routes to `router.tsx`. Activate the existing Agents item in the left navigation. Empty-state CTA on `/agents` deep-links to `/agents/new`.

13. **Settings cross-checks** — Extend `useSettings`/`useProviderKeys` to expose which providers have a stored key. Render the inline warning + `Go to Settings` link from spec §8 when the chosen provider has no key configured globally and no per-agent override.

## Phase 4 — Verification

14. **Tests** — Implement the unit + integration + frontend tests enumerated in spec §10. Backend integration uses the same disposable Postgres pattern established in F01; frontend uses MSW handlers for the new endpoints.

15. **Manual end-to-end smoke** — With the full stack running: create an agent → reload and confirm persistence → edit it → clone it → attempt a duplicate name (expect inline error) → set a per-agent key (verify masking) → switch the provider to one without a key (verify warning) → delete it (verify modal + final disappearance). Capture the steps in `docs/F02-agent-management/verify.md`.
