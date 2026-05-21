# F01. App Foundation and Settings — Implementation Plan

## Prerequisites

- Docker and Docker Compose installed locally
- Rust toolchain (stable, edition 2024) with `cargo` and `sqlx-cli`
- Node.js (LTS) and `pnpm` (or `npm`)
- This plan generates the entire scaffolding from scratch — no existing code is expected

## Phase 1 — Repository & Infrastructure Scaffolding

1. **Monorepo skeleton** — Create the top-level structure with `/backend`, `/frontend`, `docker-compose.yml`, `.env.example`, `.gitignore`, and `README.md`. Establish that the root is where `docker compose` runs and where both subprojects live side by side.

2. **PostgreSQL + pgvector via docker-compose** — Author `docker-compose.yml` with a single `postgres` service using the `pgvector/pgvector:pg16` image, a named volume for persistence, a `pg_isready` healthcheck, and port `5432` bound to `127.0.0.1`. Document the canonical `docker compose up -d` / `down` commands in the README.

3. **Environment configuration** — Define `.env.example` with `DATABASE_URL`, `BIND_ADDR`, `RUST_LOG`, and `AGENT_MAKER_HOME` (defaults to `~/.agent-maker`). Wire the backend to read these via a typed `Config` struct.

## Phase 2 — Backend Foundation

4. **Cargo project + dependencies** — Initialize the `/backend` crate with Axum, Tokio, SQLx (with `postgres`, `runtime-tokio-rustls`, `migrate`, `uuid`, `chrono`), `serde`, `tracing`, `tracing-subscriber`, `keyring`, `aes-gcm`, `argon2`, `rand`, `thiserror`, and the `rig` LLM framework. Add `reqwest` for the per-provider HTTP calls used by `test()`.

5. **Telemetry and error envelope** — Set up `tracing-subscriber` (pretty in dev, JSON in release) and a shared `AppError` type that converts into the JSON error envelope defined in the spec. Wire request-id middleware so every log line carries the correlation id.

6. **Database bootstrap** — Implement `db::init`: build the `PgPool`, retry a connection probe until ready (30s window), then run `sqlx::migrate!()`. Author the two migration files (`0001_init.sql` and `0002_pgvector.sql`) per the schema in the spec.

7. **Secret store abstraction** — Define the `SecretStore` trait and the two implementations: `KeyringStore` (using the `keyring` crate) and `FileStore` (AES-256-GCM with an Argon2-derived master key persisted at `~/.agent-maker/master.key` mode `0600`). Provide a `SecretStore::auto()` constructor that prefers `KeyringStore` and falls back to `FileStore` with a single WARN log.

8. **LLM provider abstraction** — Define the `LlmProvider` trait with `chat_stream`, `embed`, `test`, and `available_models`. Implement three modules — `anthropic`, `openai`, `openai_compat` — using `rig` clients. Each implementation pulls its API key from `SecretStore` lazily on each call. F01 only exercises `test()`; the other methods are stubbed but compile so F07/F08 can fill them in.

9. **Settings service and routes** — Implement `settings::service` for non-secret CRUD against the `settings` and `provider_keys` tables, plus key save/delete that delegate to `SecretStore`. Mount the seven REST handlers (`GET`, `PUT`, `PUT key`, `DELETE key`, `POST test`, `POST wipe`, `GET health`) under `/api`. Make sure the GET response always masks keys server-side; the raw key never leaves the server after a save.

10. **Static-file serving in release** — In release builds, mount a `tower-http` `ServeDir` at `/` that serves `/frontend/dist`, with a fallback to `index.html` for SPA routes. In dev, this fallback is inert because Vite hosts the frontend on port 5173.

## Phase 3 — Frontend Foundation

11. **Vite + React + TypeScript scaffold** — Initialize `/frontend` with Vite (React + TS template). Configure the dev proxy in `vite.config.ts` so `/api` and `/sse` forward to `http://127.0.0.1:8787`. Add Tailwind, configure `tailwind.config.ts`, and install `@tanstack/react-query`, `react-router-dom`, `zod`, and `lucide-react`.

12. **shadcn/ui setup** — Initialize shadcn/ui into the project, generating the base components needed by Settings: `button`, `input`, `label`, `card`, `dialog`, `tabs`, `select`, `switch`, `toast`. Apply the dark-mode class strategy compatible with the theme switcher.

13. **Layout, router, and query client** — Build the persistent `Layout` with a left navigation sidebar (placeholders for Agents, Skills, Templates, Conversations, Settings), wire the router, and create a singleton `QueryClient`. Add a `useSettings` hook that fetches `/api/settings`.

14. **Onboarding flow** — Implement the `/onboarding` page shown when no providers are configured. After the first successful key save, navigate the user to `/settings/providers`.

15. **Settings page** — Implement the four sections — Providers, Memory Defaults, Appearance, Data — as sub-routes under `/settings`. Use TanStack Query mutations against the REST endpoints; show shadcn toasts for save/test/delete; render masked keys with a show/hide toggle that only affects the masked string (raw key is never client-side after save). The "Test connection" button hits `POST .../test` and renders success or the provider-specific error inline. The Data section exposes the wipe action with a confirmation modal that requires typing `WIPE`.

## Phase 4 — Verification

16. **Unit and integration tests** — Implement the unit and integration tests enumerated in the spec's Testing Strategy. Run the backend test suite against a disposable Postgres (testcontainers or the docker-compose service) and the frontend test suite via Vitest + Testing Library.

17. **Manual end-to-end smoke** — Bring up the stack with `docker compose up -d && cargo run` + `pnpm dev`. Walk the golden path: onboarding → save key → test connection → adjust memory defaults → switch theme → wipe data. Confirm the "Database unavailable" screen by stopping the Postgres container and reloading. Document the steps in a short `verify.md` for future regression checks.
