# agent-maker

An operating system for personal AI. Build custom AI agents without writing code,
attach reusable skills to extend them, chat with streaming responses, and let a
memory system keep conversations coherent across weeks.

Each **agent** is a persona — a name, a preamble, and a system prompt — running on
the LLM provider and model you choose. **Skills** are small instruction bundles
(name + description + body) you attach to any number of agents, snapping in like
LEGO blocks to extend behavior across your whole roster. A per-agent **memory
system** keeps the most recent turns verbatim and semantically recalls older
relevant turns so long-running conversations stay coherent.

It is **local-first and BYOK**: the backend binds to localhost in local dev and you
bring your own provider keys. A **Clerk login wall** gates access to a single shared
workspace (data is not isolated per user), so the app can also run on a deployed
origin. See [`docs/PRD.md`](docs/PRD.md) for the full product vision.

## Features

- **Multi-provider** — Anthropic (Claude), OpenAI (GPT), and local OpenAI-compatible
  endpoints (Ollama, LM Studio) behind one abstraction; pick or switch the engine
  per agent.
- **Agents & skills** — full CRUD plus clone; edit a skill once and the change
  propagates to every agent that uses it.
- **Skill attachment** — many-to-many, drag-to-reorder; the order controls how skill
  bodies are concatenated into the system prompt.
- **Starter template library** — 10+ ready-made agents and skills across writing,
  research, productivity, coding, learning, and wellbeing; adopt one click to an
  editable copy.
- **Conversations** — multiple independent threads per agent, auto-titled, fully
  persisted between sessions.
- **Streaming chat** — token-by-token over Server-Sent Events, stop/retry, markdown
  with syntax-highlighted code.
- **Memory** — recent-N verbatim window plus top-K semantic recall via `pgvector`,
  with a "what I recalled" indicator and per-conversation memory clearing.
- **Sub-agents** — attach existing agents as sub-agents with an `@handle`; type
  `@alias` in chat to delegate that turn to a specialist (its own persona, provider,
  and model), which replies as a labeled turn before the parent synthesizes the
  final answer. Up to 10 per agent, 3 mentions per turn, self-attach and cycles
  rejected.
- **Authentication** — a Clerk email/password login wall (sign-in + sign-up) over a
  shared workspace; every `/api` route is verified against Clerk's JWKS. Auth is
  enforced only when `CLERK_JWKS_URL` and `CLERK_ISSUER` are set, so local dev runs
  without it.
- **i18n** — English and Brazilian Portuguese (`en`, `pt-BR`), with localized
  templates and a per-agent response-language override.

## Stack

- **Backend:** Rust 2024 + Axum 0.8 + SQLx (Postgres) + Tokio. Multi-provider LLM
  access via a custom abstraction in `backend/src/llm/`; streaming over SSE.
- **Frontend:** React 18 + Vite + TypeScript + Tailwind + TanStack Query +
  shadcn/ui (Radix). i18n via i18next. Package manager: **pnpm**.
- **Database:** PostgreSQL 16 with the `pgvector` extension (via Docker Compose).
- **Secrets:** OS keychain (`keyring` crate) with AES-256-GCM encrypted file
  fallback — see
  [`docs/F01-app-foundation-and-settings/secret-store.md`](docs/F01-app-foundation-and-settings/secret-store.md).
- **Deploy:** Railway (`backend/railway.toml`, `frontend/railway.toml`) — see
  [`docs/deployment.md`](docs/deployment.md).

## Repository layout

```
.
├── docker-compose.yml      # Postgres + pgvector (+ backend/frontend for full-stack)
├── .env.example            # Copy to .env and adjust
├── backend/                # Rust/Axum API
│   ├── src/                # routes/ + per-domain model.rs/service.rs/mod.rs
│   └── migrations/         # SQLx migrations, applied on startup
├── frontend/               # React/Vite SPA (pages/, hooks/, components/, i18n/)
└── docs/                   # PRD + per-feature spec & plan (F01–F11)
```

## Prerequisites

- Docker + Docker Compose
- Rust toolchain (stable) with `cargo`
- Node.js LTS + `pnpm`

## Getting started

```bash
# 1. Configure environment
cp .env.example .env

# 2. Start PostgreSQL (with pgvector). Migrations run on backend startup.
docker compose up -d

# 3. Run the backend (serves on 127.0.0.1:8787)
cd backend && cargo run

# 4. Run the frontend (Vite dev server on 127.0.0.1:5173)
cd frontend && pnpm install && pnpm dev
```

Open the frontend, add at least one provider key in **Settings → Providers**, and
create your first agent (or adopt a starter template).

Alternatively, run the whole stack in containers:

```bash
docker compose up -d            # postgres + backend + frontend
```

Postgres is bound to `127.0.0.1:5432` only — never expose it to the network. Data
persists in the named volume `agent_maker_pgdata`; remove it with
`docker compose down -v` to start fresh.

## Development

```bash
# Backend
cd backend
cargo test                      # all tests
cargo test <name>               # a single test (substring match)
cargo clippy --all-targets

# Frontend
cd frontend
pnpm test                       # vitest
pnpm typecheck                  # tsc --noEmit
pnpm build                      # typecheck + vite build
```

`AGENT_MAKER_FORCE_FILE_STORE=1` skips the OS keychain and uses the encrypted file
store — recommended on Linux desktops where Secret Service is unreliable.

## Documentation

- Product requirements: [`docs/PRD.md`](docs/PRD.md)
- Per-feature specs and plans: [`docs/`](docs/) (F01–F11)
- Deployment: [`docs/deployment.md`](docs/deployment.md)
- Contributor guidance for Claude Code: [`CLAUDE.md`](CLAUDE.md)
