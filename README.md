# agent-maker

An operating system for personal AI. Build custom agents, attach reusable skills, chat with them, and let a memory system keep conversations coherent across weeks.

> **Status:** Phase 1 — scaffolding only. Backend and frontend code are added in later phases.

## Stack

- **Backend:** Rust + Axum + SQLx + `rig` (multi-provider LLM)
- **Frontend:** React + Vite + TypeScript + Tailwind + TanStack Query + shadcn/ui
- **Database:** PostgreSQL 16 with `pgvector` (via Docker Compose)
- **Streaming:** Server-Sent Events
- **Secrets:** OS keychain (`keyring` crate) with AES-256-GCM file fallback

## Repository layout

```
.
├── docker-compose.yml      # Postgres + pgvector service
├── .env.example            # Copy to .env and adjust
├── backend/                # Rust/Axum API (added in Phase 2)
└── frontend/               # React/Vite SPA (added in Phase 3)
```

## Prerequisites

- Docker + Docker Compose
- Rust toolchain (stable) with `cargo` — added in Phase 2
- Node.js LTS + `pnpm` — added in Phase 3

## Getting started

```bash
# 1. Configure environment
cp .env.example .env

# 2. Start PostgreSQL (with pgvector)
docker compose up -d

# 3. Verify Postgres is healthy
docker compose ps

# 4. Stop when done
docker compose down
```

Postgres is bound to `127.0.0.1:5432` only — never exposed to the network. Data persists in the named volume `agent_maker_pgdata`; remove it with `docker compose down -v` to start fresh.

## Documentation

- Product requirements: [`docs/PRD.md`](docs/PRD.md)
- F01 spec and plan: [`docs/F01-app-foundation-and-settings/`](docs/F01-app-foundation-and-settings/)
