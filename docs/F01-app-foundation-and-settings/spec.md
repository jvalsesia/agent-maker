# F01. App Foundation and Settings — Technical Specification

## 1. Overview

F01 establishes the entire technical foundation of agent-maker: the monorepo layout, the Rust + Axum backend, the React + Vite frontend, the PostgreSQL + pgvector database provisioned via docker-compose, the full database schema (used by every subsequent feature), the multi-provider LLM abstraction built on `rig`, the encrypted local key store, and a single-page Settings UI that lets the user configure providers, defaults, theme, and data management. Every other feature (F02–F08) consumes this foundation.

## 2. Scope

**Included:**
- Monorepo scaffolding: `/backend` (Rust/Axum), `/frontend` (React/Vite/TS/Tailwind), `/docker-compose.yml` at the root
- Bundled `docker-compose.yml` running PostgreSQL with the `pgvector/pgvector` image, named volume, healthcheck, port `5432`
- Database schema covering: `agents`, `skills`, `agent_skills`, `conversations`, `messages`, `message_embeddings`, `settings`, `provider_keys`
- SQLx migration runner that executes pending migrations at backend startup (waits on Postgres healthcheck)
- Multi-provider LLM abstraction layer using the `rig` Rust framework, wrapping Anthropic, OpenAI, and a generic OpenAI-compatible endpoint (Ollama / LM Studio); unified surface = `chat_stream(...)` and `embed(...)`
- Encrypted key storage: OS keychain via the `keyring` crate when available; fallback to an AES-256-GCM encrypted file at `~/.agent-maker/secrets.bin` with an Argon2-derived master key at `~/.agent-maker/master.key` (mode `0600`)
- REST settings API under `/api/settings/*`
- Backend bound to `127.0.0.1:8787` (no external exposure)
- Frontend SPA with persistent left navigation; Settings page with sections: Providers, Memory Defaults, Appearance, Data
- Onboarding screen on first launch when no provider key is configured
- "Test connection" per provider performs a real, minimal LLM call (cheap model, 1 token max) and reports success/failure
- Structured logging via `tracing` + `tracing-subscriber` (JSON in release, pretty in dev)
- Wipe-local-data action removes all DB rows AND clears the key store

**Integrated from PRD blocks:**
- `Provides` (input to F02–F08): persistence layer + configured LLM clients + default keys
- `Capabilities`: settings page, masked keys, OS keychain + encrypted file fallback, healthcheck wait, multi-provider abstraction, default N=10/K=5
- `Experience`: onboarding screen, left-nav settings, masked-with-show toggle, auto-save toast
- `Error Handling`: DB init failure, API test failure, keychain unavailable warning, save failure
- Section 9 acceptance criteria for F01

**Deferred (out of F01):**
- Any agent / skill / conversation / chat UI (F02–F07)
- Embedding generation logic and ANN index population (F08 — F01 only creates the schema and index DDL)

## 3. Component Overview

### Repository layout

```
/agent-maker
├── docker-compose.yml
├── .env.example
├── README.md
├── /backend
│   ├── Cargo.toml
│   ├── /migrations
│   │   ├── 0001_init.sql
│   │   └── 0002_pgvector.sql
│   └── /src
│       ├── main.rs
│       ├── config.rs
│       ├── db.rs
│       ├── error.rs
│       ├── routes/
│       │   ├── mod.rs
│       │   └── settings.rs
│       ├── secrets/
│       │   ├── mod.rs
│       │   ├── keyring_store.rs
│       │   └── file_store.rs
│       ├── llm/
│       │   ├── mod.rs
│       │   ├── provider.rs       // trait LlmProvider
│       │   ├── anthropic.rs
│       │   ├── openai.rs
│       │   └── openai_compat.rs  // Ollama / LM Studio
│       ├── settings/
│       │   ├── mod.rs
│       │   ├── model.rs
│       │   └── service.rs
│       └── telemetry.rs
└── /frontend
    ├── package.json
    ├── vite.config.ts
    ├── tailwind.config.ts
    ├── index.html
    └── /src
        ├── main.tsx
        ├── App.tsx
        ├── router.tsx
        ├── lib/
        │   ├── api.ts            // fetch wrapper
        │   └── queryClient.ts    // TanStack Query
        ├── components/
        │   ├── ui/               // shadcn/ui generated
        │   ├── Layout.tsx
        │   └── NavSidebar.tsx
        ├── pages/
        │   ├── Onboarding.tsx
        │   └── Settings/
        │       ├── index.tsx
        │       ├── Providers.tsx
        │       ├── MemoryDefaults.tsx
        │       ├── Appearance.tsx
        │       └── Data.tsx
        └── hooks/
            └── useSettings.ts
```

### Key modules

| Module | Responsibility |
|---|---|
| `backend/src/main.rs` | Build the Axum app, mount routes, bind `127.0.0.1:8787`, in release mode serve `/frontend/dist` as static files |
| `backend/src/db.rs` | Build `PgPool`, wait for healthcheck, run `sqlx::migrate!()` on startup |
| `backend/src/secrets/` | `SecretStore` trait with two implementations: `KeyringStore` (preferred) and `FileStore` (AES-256-GCM fallback). One-time warning logged when falling back |
| `backend/src/llm/provider.rs` | `LlmProvider` trait: `chat_stream`, `embed`, `test`, `available_models` |
| `backend/src/llm/{anthropic,openai,openai_compat}.rs` | Per-provider implementations wrapping `rig` clients; each loads its API key from `SecretStore` on demand |
| `backend/src/settings/service.rs` | Read/write non-secret config (default provider, default model, theme, memory N/K) in the `settings` table; orchestrate key write/delete via the `SecretStore` |
| `backend/src/routes/settings.rs` | REST handlers under `/api/settings/*`; returns DTOs with masked keys only |
| `frontend/src/pages/Settings/*` | UI sections; uses TanStack Query against `/api/settings/*`; shadcn dialogs/forms/toasts |
| `frontend/src/pages/Onboarding.tsx` | Shown when `GET /api/settings` returns zero configured providers; offers per-provider quick links and the same form as `Providers.tsx` |

### Process lifecycle

1. Developer runs `docker compose up -d` → Postgres + pgvector starts; named volume persists.
2. Developer runs `cargo run` (or built binary) → backend reads `DATABASE_URL` from `.env` (default matches compose service), waits for Postgres readiness (up to 30s, 1s interval), runs migrations, initializes `SecretStore`, mounts routes, listens on `127.0.0.1:8787`.
3. In dev, `pnpm dev` (frontend) runs Vite on `5173` with a proxy: `/api` → `http://127.0.0.1:8787`. In release, the backend serves `/frontend/dist` directly, so a single binary suffices.
4. On first frontend load, the app calls `GET /api/settings`; if zero providers have configured keys, the router redirects to `/onboarding`.

## 4. API Contracts

All endpoints are served at `http://127.0.0.1:8787`. Responses are JSON. Errors use a uniform envelope:

```json
{ "error": { "code": "invalid_api_key", "message": "Provider rejected the key (401).", "provider": "anthropic" } }
```

### `GET /api/settings`

Returns the current configuration. API keys are **masked**.

```json
{
  "default_provider": "anthropic",
  "default_model": { "anthropic": "claude-sonnet-4-6", "openai": "gpt-4o", "openai_compat": null },
  "memory_defaults": { "recent_n": 10, "top_k": 5 },
  "appearance": { "theme": "system" },
  "providers": [
    { "name": "anthropic", "key_configured": true, "key_masked": "sk-ant-***...A1B2", "base_url": null },
    { "name": "openai",    "key_configured": true, "key_masked": "sk-***...9F0E",    "base_url": null },
    { "name": "openai_compat", "key_configured": false, "key_masked": null, "base_url": "http://localhost:11434/v1" }
  ],
  "key_store_backend": "keyring"
}
```

### `PUT /api/settings`

Updates non-secret fields. Partial updates allowed.

Request:
```json
{
  "default_provider": "openai",
  "default_model": { "anthropic": "claude-haiku-4-5" },
  "memory_defaults": { "recent_n": 12, "top_k": 6 },
  "appearance": { "theme": "dark" }
}
```

Response: `200` with the same shape as `GET /api/settings`.

Validation:
- `default_provider ∈ {anthropic, openai, openai_compat}`
- `recent_n ∈ [4, 30]`, `top_k ∈ [0, 10]`
- `theme ∈ {light, dark, system}`

### `PUT /api/settings/providers/:name/key`

Saves or replaces a provider's API key (and optional base_url for `openai_compat`).

Request:
```json
{ "key": "sk-ant-api03-...", "base_url": "http://localhost:11434/v1" }
```

Response: `204 No Content`. The key is written to the `SecretStore`; the `provider_keys` table stores only metadata (mask, created_at, updated_at).

### `DELETE /api/settings/providers/:name/key`

Removes the stored key from the `SecretStore` and the corresponding `provider_keys` row. Response: `204`.

### `POST /api/settings/providers/:name/test`

Performs a real, minimal LLM call against the provider using the stored key.

Response on success:
```json
{ "ok": true, "model_used": "claude-haiku-4-5", "latency_ms": 412 }
```

Response on failure (`400`):
```json
{ "error": { "code": "invalid_api_key", "message": "Anthropic returned 401: invalid x-api-key", "provider": "anthropic" } }
```

Implementation: the handler dispatches to the provider's `test()` method, which sends a 1-token, cheapest-model completion (or an `embeddings` request for `openai_compat` if the endpoint does not implement chat). Timeout 10s.

### `POST /api/settings/data/wipe`

Wipes all local data: truncates every DB table and clears every stored secret. Requires a confirmation body to prevent accidents.

Request:
```json
{ "confirm": "WIPE" }
```

Response: `204`. Backend logs the action at WARN level.

### `GET /api/health`

Returns `200 {"status":"ok","db":"ok","key_store":"keyring"}` once DB is reachable and migrations have run. Used by the frontend bootstrap to detect "DB unavailable" failure mode.

## 5. Data Model

All tables in the `public` schema. Postgres 16+ with `pgvector` extension.

### Migration `0001_init.sql`

```sql
-- Extensions (pgvector enabled in migration 0002 to keep this file portable)

-- settings: single-row key/value-ish table (PK = 'singleton')
CREATE TABLE settings (
    id              TEXT PRIMARY KEY DEFAULT 'singleton' CHECK (id = 'singleton'),
    default_provider        TEXT        NOT NULL DEFAULT 'anthropic',
    default_model_anthropic TEXT,
    default_model_openai    TEXT,
    default_model_openai_compat TEXT,
    recent_n        SMALLINT    NOT NULL DEFAULT 10 CHECK (recent_n BETWEEN 4 AND 30),
    top_k           SMALLINT    NOT NULL DEFAULT 5  CHECK (top_k     BETWEEN 0 AND 10),
    theme           TEXT        NOT NULL DEFAULT 'system' CHECK (theme IN ('light','dark','system')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO settings (id) VALUES ('singleton');

-- provider_keys: metadata only; actual secret lives in the OS keychain or encrypted file
CREATE TABLE provider_keys (
    name        TEXT PRIMARY KEY CHECK (name IN ('anthropic','openai','openai_compat')),
    key_masked  TEXT NOT NULL,
    base_url    TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- agents (consumed by F02; F01 only creates schema)
CREATE TABLE agents (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL UNIQUE,
    preamble     TEXT,
    system_prompt TEXT NOT NULL,
    provider     TEXT NOT NULL CHECK (provider IN ('anthropic','openai','openai_compat')),
    model        TEXT NOT NULL,
    has_override_key BOOLEAN NOT NULL DEFAULT FALSE,
    recent_n_override SMALLINT CHECK (recent_n_override BETWEEN 4 AND 30),
    top_k_override    SMALLINT CHECK (top_k_override BETWEEN 0 AND 10),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX idx_agents_last_used_at ON agents (last_used_at DESC NULLS LAST);

-- skills (consumed by F03)
CREATE TABLE skills (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT NOT NULL UNIQUE,
    description  TEXT NOT NULL,
    body         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- agent_skills (consumed by F04): many-to-many with order
CREATE TABLE agent_skills (
    agent_id   UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    skill_id   UUID NOT NULL REFERENCES skills (id) ON DELETE CASCADE,
    position   SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 19),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, skill_id),
    UNIQUE (agent_id, position)
);
CREATE INDEX idx_agent_skills_skill ON agent_skills (skill_id);

-- conversations (consumed by F06)
CREATE TABLE conversations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id        UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    title           TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    message_count   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_conversations_agent_activity ON conversations (agent_id, last_activity_at DESC);

-- messages (consumed by F06, F07, F08)
CREATE TABLE messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'complete' CHECK (status IN ('complete','stopped','error')),
    model           TEXT,
    token_count     INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_messages_conversation_created ON messages (conversation_id, created_at);
```

### Migration `0002_pgvector.sql`

```sql
CREATE EXTENSION IF NOT EXISTS vector;

-- One embedding row per indexed message. Embedding dimension is provider-dependent;
-- we store the dim explicitly to allow mixed providers per conversation.
CREATE TABLE message_embeddings (
    message_id  UUID PRIMARY KEY REFERENCES messages (id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    model       TEXT NOT NULL,
    dim         INTEGER NOT NULL,
    embedding   vector(1536) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_message_embeddings_conv ON message_embeddings (conversation_id);
CREATE INDEX idx_message_embeddings_ann
    ON message_embeddings USING hnsw (embedding vector_cosine_ops);
```

Note on `vector(1536)`: defaults to the OpenAI `text-embedding-3-small` size. F08 will revisit if multi-dimension support is needed; F01 sets the default for the schema.

### Settings DTO ↔ table mapping

| API field | Source |
|---|---|
| `default_provider`, `default_model.*`, `memory_defaults.*`, `appearance.theme` | `settings` table |
| `providers[].key_configured`, `key_masked`, `base_url` | `provider_keys` table |
| `key_store_backend` | runtime: `"keyring"` or `"file"` based on which `SecretStore` impl is active |

### docker-compose.yml

```yaml
services:
  postgres:
    image: pgvector/pgvector:pg16
    container_name: agent-maker-postgres
    restart: unless-stopped
    environment:
      POSTGRES_DB: agentmaker
      POSTGRES_USER: agentmaker
      POSTGRES_PASSWORD: agentmaker
    ports:
      - "127.0.0.1:5432:5432"
    volumes:
      - agent_maker_pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U agentmaker -d agentmaker"]
      interval: 2s
      timeout: 2s
      retries: 30

volumes:
  agent_maker_pgdata:
```

## 6. Error Handling

| Scenario | Backend behavior | Frontend behavior |
|---|---|---|
| Postgres unreachable on startup | Retry healthcheck up to 30s; on failure, log ERROR with connection string and exit code 1 | If the SPA loads but `GET /api/health` returns DNS/refused, show full-screen "Database unavailable" with the configured DSN and the `docker compose up -d` remediation hint |
| Migration fails | Log ERROR with migration name, exit 1 | Same full-screen error as above |
| OS keychain unavailable | Detect at startup; log WARN once; fall back to `FileStore`; expose `key_store_backend: "file"` in `GET /api/settings` | Settings → Providers shows a one-time banner: "Using encrypted file fallback — OS keychain unavailable" |
| `PUT .../key` write fails (keychain error, disk full) | Return `500` with `code: "key_write_failed"` | Toast with reason; form stays dirty |
| `POST .../test` fails: provider 4xx | Return `400` with provider's status code and body excerpt in the message | Inline error under the provider section with the verbatim message |
| `POST .../test` fails: network/timeout | Return `400` with `code: "test_unreachable"` | Inline error with retry button |
| Invalid `PUT /api/settings` body (out-of-range N/K, unknown provider) | Return `400` with `code: "validation_error"`, field path, expected range | Inline field-level error |
| Wipe missing confirmation | Return `400` with `code: "confirmation_required"` | Modal cannot submit until user types `WIPE` |
| Duplicate concurrent settings writes | Last-write-wins on `settings` row (no optimistic locking in v1) | None |

All errors are logged via `tracing` with the request id, route, status, and error code.

## 7. Testing Strategy

### Unit tests (backend)

- `settings::service::tests` — validates range checks for `recent_n`, `top_k`, allowed provider values, theme values
- `secrets::file_store::tests` — round-trip encrypt/decrypt with a fresh master key; corrupted ciphertext returns an error; missing master key generates one with `0600` permissions
- `secrets::tests::keyring_to_file_fallback` — when `KeyringStore::new()` returns `Err`, `SecretStore::auto()` constructs a `FileStore` and logs the warning exactly once
- `llm::provider::tests::masking` — `Provider::mask_key("sk-ant-api03-XYZ123ABC")` returns `"sk-ant-***...3ABC"` (first 7 + last 4)

### Integration tests (backend)

Use `sqlx::test` with a per-test Postgres database (spin up via testcontainers or via the docker-compose service in CI). Each test:

- `test_health_after_migrations` — `GET /api/health` returns `200` after `db::init` runs
- `test_settings_default_state` — `GET /api/settings` on a fresh DB returns defaults: `default_provider="anthropic"`, `recent_n=10`, `top_k=5`, `theme="system"`, all providers `key_configured=false`
- `test_settings_put_partial` — `PUT /api/settings` with `{ "appearance": { "theme": "dark" } }` updates only theme; subsequent `GET` reflects the change
- `test_settings_put_validation` — `PUT /api/settings` with `recent_n=2` returns `400` `validation_error`
- `test_provider_key_save_then_get` — `PUT /api/settings/providers/anthropic/key` followed by `GET /api/settings` returns `key_configured=true` and `key_masked` matches the `mask_key` rule; the raw key never appears in the response
- `test_provider_key_delete` — after `DELETE`, `GET` returns `key_configured=false`
- `test_provider_test_invalid_key` — `POST /api/settings/providers/anthropic/test` with a wrong key returns `400` with the provider's 401 message (uses `mockito` to stub the provider HTTP server)
- `test_provider_test_success` — same call with mockito returning a valid completion returns `200 { ok: true, model_used, latency_ms }`
- `test_wipe_clears_db_and_secrets` — after saving keys and creating dummy rows, `POST /api/settings/data/wipe` with `confirm:"WIPE"` empties every table and removes secrets from the `SecretStore`
- `test_pgvector_extension_loaded` — `SELECT extname FROM pg_extension WHERE extname='vector'` returns one row

### Frontend tests

- `Settings/Providers.test.tsx` — renders the masked key when configured; "show" toggle reveals the masked string only (raw key is never present in the DOM because the backend never sends it)
- `useSettings.test.ts` — TanStack Query refetches `/api/settings` after a successful `PUT`
- `Onboarding.test.tsx` — renders when `GET /api/settings` returns zero configured providers; redirects to `/` once at least one is saved
- Component snapshot for theme switching (light/dark/system) applies the corresponding Tailwind class on `<html>`

### Acceptance tests (mapped from PRD Section 9)

| PRD acceptance criterion | Test |
|---|---|
| `docker compose up -d` starts Postgres with pgvector; first launch runs migrations and shows onboarding | `test_health_after_migrations` + `Onboarding.test.tsx` |
| If Postgres unreachable, app shows error with DSN and remediation hint | Manual test (covered in `verify.md` checklist): stop the compose service, reload, assert the error screen text |
| User can paste a key, save, and a "test connection" returns success when valid | `test_provider_key_save_then_get` + `test_provider_test_success` |
| Masked-by-default with "show" reveals; reloading preserves saved (encrypted) keys | `Settings/Providers.test.tsx` + `test_provider_key_save_then_get` |
| Selecting a default provider and model persists across reloads | `test_settings_put_partial` |
| Invalid API keys return a clear, provider-specific error on test | `test_provider_test_invalid_key` |
| Wiping local data removes all rows and embeddings after confirmation | `test_wipe_clears_db_and_secrets` |

### Cross-feature integration (forward-looking)

Although F01 is implemented first, its Provides surface is what F02–F08 will consume. Integration smoke tests living alongside F01 verify the schema is queryable:

- Insert a dummy `agent`, `skill`, `agent_skill`, `conversation`, `message`, and `message_embedding`, then read them back through SQL — confirms migrations match the documented shape.

## 8. Assumptions and Decisions

The following choices were made during the interview (no PRD answer existed):

- **Backend stack:** Rust + Axum + SQLx (user-confirmed)
- **Frontend stack:** React + Vite + TypeScript + Tailwind + TanStack Query + shadcn/ui (user-confirmed)
- **Streaming:** Server-Sent Events (user-confirmed, also matches PRD F07)
- **Repo layout:** monorepo with `/backend` and `/frontend` (user-confirmed)
- **Settings API:** REST under `/api/settings/*` (recommendation accepted)
- **Key storage:** `keyring` crate with AES-256-GCM file fallback (recommendation accepted, matches PRD)
- **Migrations tool:** `sqlx migrate` built-in runner (user-confirmed)
- **LLM library:** `rig` Rust framework wrapping Anthropic, OpenAI, and OpenAI-compatible endpoints (user-confirmed)
- **Networking:** backend on `127.0.0.1:8787`; Vite dev proxy in development; Axum serves `/frontend/dist` in production (recommendation accepted)
- **Embedding dimension:** schema defaults to `vector(1536)` (OpenAI `text-embedding-3-small`); F08 will confirm
- **ANN index:** HNSW with `vector_cosine_ops` (industry default for high-recall semantic search)
- **Default Postgres credentials:** `agentmaker` / `agentmaker` / `agentmaker` exposed on `127.0.0.1:5432`. Documented as local-dev defaults; user can override via `.env`
- **Master-key fallback path:** `~/.agent-maker/master.key`, mode `0600`; secrets file `~/.agent-maker/secrets.bin`
