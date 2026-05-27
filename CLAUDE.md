# CLAUDE.md

Guidance for Claude Code when working in this repository.

## What this is

**agent-maker** is an "operating system for personal AI": users create custom AI
agents (persona = name + preamble + prompt), build reusable **skills** (instruction
bundles attachable to many agents), and chat with them. A **memory system** keeps
per-agent conversation history coherent over time by surfacing recent turns and
recalling older relevant ones via semantic (pgvector) similarity.

See `project-intent.md` and `docs/PRD.md` for the product vision.

## Stack

- **Backend:** Rust 2024 edition + Axum 0.8 + SQLx (Postgres) + Tokio. LLM access is
  multi-provider (OpenAI / Anthropic / OpenAI-compatible) via a custom provider
  abstraction in `backend/src/llm/`. Streaming over Server-Sent Events.
- **Frontend:** React 18 + Vite + TypeScript + Tailwind + TanStack Query +
  shadcn/ui (Radix). i18n via i18next (en, pt-BR). Package manager: **pnpm**.
- **Database:** PostgreSQL 16 with `pgvector` (Docker Compose).
- **Secrets:** OS keychain (`keyring`) with AES-256-GCM encrypted file fallback.
  Set `AGENT_MAKER_FORCE_FILE_STORE=1` on Linux where Secret Service is unreliable.
- **Deploy:** Railway (`backend/railway.toml`, `frontend/railway.toml`), Docker images.

## Repository layout

```
backend/        Rust/Axum API
  src/
    routes/         HTTP handlers (agents, skills, attachments, conversations,
                    templates, memory, settings, chat)
    <domain>/       Per-domain mod.rs + model.rs + service.rs
                    (agents, skills, conversations, memory, chat, settings,
                     templates, skill_attachments, secrets, llm)
  migrations/       SQLx migrations (0001_init … 0006_i18n)
frontend/
  src/
    pages/          Route screens (Agents, Skills, Templates, Settings, Chat, Onboarding)
    hooks/          TanStack Query data hooks (useAgents, useChat, useMemory, …)
    components/ui/   shadcn primitives
    i18n/           i18next setup + locales/{en,pt-BR}.json
docs/             PRD + per-feature spec/plan (F01–F09)
```

The backend follows a consistent per-domain pattern: `model.rs` (types/DB rows),
`service.rs` (business logic), `mod.rs` (wiring); HTTP lives separately in `routes/`.

## Common commands

```bash
# Database (required for backend + tests)
docker compose up -d                     # Postgres + pgvector on 127.0.0.1:5432

# Backend
cd backend
cargo run                                # serves on 127.0.0.1:8787 (BIND_ADDR)
cargo test
cargo clippy --all-targets

# Frontend
cd frontend
pnpm install
pnpm dev                                 # Vite dev server
pnpm test                                # vitest
pnpm typecheck                           # tsc --noEmit
pnpm build                               # typecheck + vite build

# Full stack via containers
docker compose up -d                     # postgres + backend + frontend
```

Copy `.env.example` to `.env` first. Postgres binds to `127.0.0.1` only — never
expose it. SQLx migrations in `backend/migrations/` apply on startup.

## Conventions

- Backend: idiomatic Rust — `Result` + `thiserror` for errors (`error.rs`),
  borrow over clone, prefer iterators. New domains follow the
  model/service/mod + `routes/` split above.
- Frontend: server state goes through TanStack Query hooks in `src/hooks/`, never
  ad-hoc fetches in components. All user-facing strings go through i18next — add
  keys to **both** `en.json` and `pt-BR.json`.
- Tests live beside the code (`*.test.ts(x)` for frontend, `#[cfg(test)]` /
  `tests/` for backend).

## Development workflow & skills

This project was built feature-by-feature (F01–F09, all merged to `main`) using a
spec-driven workflow backed by Claude Code skills. Reuse them rather than working
ad hoc:

| Skill | Use it for |
|-------|-----------|
| `prd-writer` | Author/extend `docs/PRD.md` — the product requirements driving every feature. |
| `spec-writer` | Turn the PRD + codebase into a per-feature spec & plan under `docs/F0x-*/`. Supports batch mode for features in the same wave. |
| `implement-feature` | Implement a feature from its spec/plan, one commit per phase, reported against acceptance criteria. |
| `rust-best-practices` | Writing/reviewing backend Rust (ownership, `Result`, performance, tests). |
| `vite-react-best-practices` | Building/reviewing the Vite + React SPA (perf, architecture, static hosting). |
| `frontend-design` | Designing distinctive, production-grade UI for new screens/components. |
| `use-railway` | Any Railway deployment, service/env/bucket, or build-failure work. See `docs/deployment.md`. |
| `claude-api` | Anthropic SDK / Claude API work and model-version migrations. |
| `code-review` / `security-review` | Review the current diff before merging. |
| `verify` / `run` | Launch the app and confirm a change works end-to-end. |
| `playwright-cli` | Browser automation / UI testing. |

Typical loop for a new feature: **prd-writer** (if scope is new) → **spec-writer**
→ **implement-feature** → **verify** → **code-review**. Lean on
`rust-best-practices` / `vite-react-best-practices` while implementing.

Per-feature specs and plans live in `docs/F0x-<name>/` — read the relevant one
before changing a feature's behavior.

## Feature map (F01–F09, on `main`)

F01 app foundation & settings · F02 agent management · F03 skill management ·
F04 skill attachment · F05 starter template library · F06 conversation management ·
F07 chat runtime · F08 memory system · F09 internationalization.

`skills-example.md` holds 10 ready-made skill fixtures for exercising the
create/clone/attach flows.
