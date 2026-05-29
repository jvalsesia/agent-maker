# Deployment

agent-maker ships as two containers — a Rust/Axum **backend** and an nginx-served
React **frontend** — backed by **PostgreSQL with the `pgvector` extension**.

- `backend/Dockerfile` — multi-stage Rust build → slim Debian runtime.
- `frontend/Dockerfile` — pnpm build → nginx (proxies `/api` to the backend).
- `docker-compose.yml` — Postgres + backend + frontend for local/self-hosted runs.
- `backend/railway.toml`, `frontend/railway.toml` — Railway service configs.

## Local / self-hosted (Docker Compose)

```bash
docker compose up --build
```

- Frontend: <http://127.0.0.1:8080>
- Backend API: <http://127.0.0.1:8787/api/health>
- Postgres: `127.0.0.1:5432` (`agentmaker` / `agentmaker`)

The backend applies migrations on startup (embedded via `sqlx::migrate!`),
including `CREATE EXTENSION vector`, which is why the database image is
`pgvector/pgvector`. The OS keychain is unavailable in containers, so the
backend runs with `AGENT_MAKER_FORCE_FILE_STORE=1` and persists its encrypted
secret store under the `/data` volume. Provider API keys are entered at runtime
through the app's Settings UI — none are baked into the image.

## Railway

The repo is a monorepo, so each service points at its subdirectory.

### 1. Provision Postgres with pgvector

Add a database from Railway's **pgvector** template (or any Postgres plugin whose
image includes the `vector` extension — the standard Railway Postgres image does
**not**). The startup migration runs `CREATE EXTENSION IF NOT EXISTS vector`, so
the extension binary must be present on the database.

### 2. Backend service

- **Root directory:** `backend`  (Railway auto-detects `backend/railway.toml` → Dockerfile build).
- **Variables:**
  - `DATABASE_URL = ${{Postgres.DATABASE_URL}}` (reference the Postgres service)
  - `AGENT_MAKER_FORCE_FILE_STORE = 1`
  - `AGENT_MAKER_HOME = /data`
  - `RUST_LOG = agent_maker=info,tower_http=info,sqlx=warn`
- **PORT:** injected by Railway and honored automatically (`Config` binds `0.0.0.0:$PORT`). Do **not** set `BIND_ADDR`.
- **Volume:** mount a volume at `/data` so the encrypted secret store survives redeploys.
- Health check (`/api/health`) is preconfigured in `railway.toml`.

### 3. Frontend service

- **Root directory:** `frontend`.
- **Variables:**
  - `BACKEND_URL = http://${{backend.RAILWAY_PRIVATE_DOMAIN}}:8787` — proxy target for `/api` (use the backend's private domain; the backend listens on the port from its own `$PORT`, so align this value with that port).
- **PORT:** injected by Railway; nginx listens on it automatically.
- Add a public domain to the **frontend** service only. Health check is `/healthz`.

> Note: the backend's `$PORT` (set by Railway) must match the port in `BACKEND_URL`.
> The simplest setup is to give the backend a fixed `PORT` (e.g. `8787`) and
> reference that same port in the frontend's `BACKEND_URL`.

## Authentication (Clerk login wall — F10)

The app gates every protected route and `/api` endpoint behind a Clerk session.
Auth is **enforced only when fully configured** so local dev and the test suite
keep working without a Clerk account:

- **Backend** enforces auth **iff both** `CLERK_JWKS_URL` and `CLERK_ISSUER` are
  set. With both unset the `require_auth` middleware is a pass-through (a warning
  is logged once). Setting **exactly one fails fast at startup** — set both or
  neither. Only `GET /api/health` stays public when auth is enabled.
- **Frontend** mounts `<ClerkProvider>` and the route guard **iff**
  `VITE_CLERK_PUBLISHABLE_KEY` is present at build time. Absent → the login wall
  is disabled on the client.

> Production checklist: to actually enforce the wall you must set **all three** —
> the backend's `CLERK_JWKS_URL` + `CLERK_ISSUER` and the frontend's
> `VITE_CLERK_PUBLISHABLE_KEY` (the latter is baked in at build time, so rebuild
> the frontend image after setting it). A prod frontend missing the publishable
> key silently disables the wall on the client.

Get the values from the Clerk dashboard for your instance: the publishable key,
the Frontend API domain (→ `CLERK_ISSUER`, e.g.
`https://<instance>.clerk.accounts.dev`), and its `/.well-known/jwks.json` URL
(→ `CLERK_JWKS_URL`). `CORS_ALLOWED_ORIGIN` is only needed when the frontend is
served from a different origin than the backend; same-origin deployments (the
nginx proxy / Vite dev proxy) leave it unset.

### Docker Compose (local full stack)

`docker-compose.yml` wires all three Clerk values from the repo-root `.env` (or
your shell env) automatically — they're optional, so an empty/missing value just
leaves auth off:

- Backend reads `CLERK_ISSUER` / `CLERK_JWKS_URL` at **runtime** (`${...:-}`
  interpolation), so a plain `docker compose up -d` picks up changes.
- The frontend's `VITE_CLERK_PUBLISHABLE_KEY` is passed as a **build arg** and
  inlined by Vite into the bundle. This means you must build, not just restart:

  ```bash
  docker compose up -d --build   # required so the frontend bakes in the key
  ```

  Rebuild the frontend image any time the publishable key changes; a restart
  alone reuses the previously built (auth-off) bundle.

The compose frontend serves on `http://localhost:8080` (nginx, proxying `/api`
to the backend), so add that origin to your Clerk instance's allowed origins for
local testing.

## Environment variables reference

| Variable | Service | Purpose | Default |
| --- | --- | --- | --- |
| `DATABASE_URL` | backend | Postgres connection string | local dev DSN |
| `PORT` | backend, frontend | Listen port (Railway injects) | 8787 / 80 |
| `BIND_ADDR` | backend | Explicit bind addr; overrides `PORT` | unset |
| `AGENT_MAKER_HOME` | backend | Data/secret-store dir | `/data` |
| `AGENT_MAKER_FORCE_FILE_STORE` | backend | Skip OS keychain (required in containers) | `1` |
| `RUST_LOG` | backend | Log filter | info |
| `CLERK_JWKS_URL` | backend | Clerk JWKS endpoint; enables auth (with `CLERK_ISSUER`) | unset (auth off) |
| `CLERK_ISSUER` | backend | Expected JWT `iss`; enables auth (with `CLERK_JWKS_URL`) | unset (auth off) |
| `CORS_ALLOWED_ORIGIN` | backend | Allowed cross-origin frontend (CORS) | unset (same-origin) |
| `VITE_CLERK_PUBLISHABLE_KEY` | frontend | Clerk publishable key (build-time); enables the client wall | unset (auth off) |
| `BACKEND_URL` | frontend | Upstream for `/api` proxy | `http://backend:8787` |
