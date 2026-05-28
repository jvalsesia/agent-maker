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

## Environment variables reference

| Variable | Service | Purpose | Default |
| --- | --- | --- | --- |
| `DATABASE_URL` | backend | Postgres connection string | local dev DSN |
| `PORT` | backend, frontend | Listen port (Railway injects) | 8787 / 80 |
| `BIND_ADDR` | backend | Explicit bind addr; overrides `PORT` | unset |
| `AGENT_MAKER_HOME` | backend | Data/secret-store dir | `/data` |
| `AGENT_MAKER_FORCE_FILE_STORE` | backend | Skip OS keychain (required in containers) | `1` |
| `RUST_LOG` | backend | Log filter | info |
| `BACKEND_URL` | frontend | Upstream for `/api` proxy | `http://backend:8787` |
| `RUST_ENV` | backend | `development` enables `DISABLE_AUTH`; anything else is treated as production | `production` (in compose) |
| `DISABLE_AUTH` | backend | Bypass auth middleware with a fake admin context. Only honored when `RUST_ENV=development` | unset |
| `FUSIONAUTH_BASE_URL` | backend | URL of the private FusionAuth service | `http://fusionauth:9011` (compose) |
| `FUSIONAUTH_TENANT_ID` | backend | Tenant UUID configured by kickstart | pinned in kickstart |
| `FUSIONAUTH_APPLICATION_ID` | backend | OAuth application UUID | pinned in kickstart |
| `FUSIONAUTH_CLIENT_ID` | backend | OAuth client id (defaults to application id) | pinned in kickstart |
| `FUSIONAUTH_CLIENT_SECRET` | backend | OAuth client secret | pinned in kickstart |
| `FUSIONAUTH_API_KEY` | backend | FusionAuth admin API key used by the backend | pinned in kickstart |
| `AUTH_COOKIE_DOMAIN` | backend | Domain attribute for session cookies; empty for localhost | unset |
| `AUTH_COOKIE_SECURE` | backend | `true`/`false`; must be `true` over HTTPS | `true` |

## F10 — Access and Identity (FusionAuth on Railway)

agent-maker authenticates users against a private FusionAuth instance. The
browser never contacts FusionAuth directly; the backend proxies the OAuth
endpoints over Railway's private network. The deployment is a **single
shared workspace** — all authenticated users see the same data — and new
accounts are created by admins via the FusionAuth admin UI.

### 1. Provision the FusionAuth database

Add a second Postgres service (Railway's stock Postgres image is fine —
no `pgvector` needed). It will hold FusionAuth's own schema. Keep it
separate from the agent-maker `Postgres` service; FusionAuth manages its
own tables and version upgrades.

### 2. Provision FusionAuth

- **Image:** `fusionauth/fusionauth-app:1.55.1` (pin the version that matches
  `docker-compose.yml`).
- **Internal-only:** do **not** add a public domain. FusionAuth must stay on
  the private network.
- **Variables:** mirror the `fusionauth` service in `docker-compose.yml`,
  pointing `DATABASE_URL` at the FusionAuth Postgres from step 1. Set
  `FUSIONAUTH_APP_RUNTIME_MODE=production`.
- **Kickstart on first boot:** mount `infra/fusionauth/kickstart/kickstart.json`
  via Railway's file mount feature, set
  `FUSIONAUTH_APP_KICKSTART_FILE=/usr/local/fusionauth/kickstart/kickstart.json`,
  and deploy. Subsequent deploys: leave the variable set but kickstart is a
  no-op once FusionAuth has been initialized — the marker file lives in
  FusionAuth's config volume.
- **Healthcheck path:** `/api/status`.

### 3. Wire the backend to FusionAuth

On the **backend** service, add:

- `RUST_ENV=production`
- `DISABLE_AUTH` — leave unset.
- `FUSIONAUTH_BASE_URL=http://${{fusionauth.RAILWAY_PRIVATE_DOMAIN}}:9011`
- `FUSIONAUTH_TENANT_ID`, `FUSIONAUTH_APPLICATION_ID`,
  `FUSIONAUTH_CLIENT_ID`, `FUSIONAUTH_CLIENT_SECRET`,
  `FUSIONAUTH_API_KEY` — copy the values from your kickstart (or rotate
  them via the FusionAuth admin UI after first boot and update the
  backend variables).
- `AUTH_COOKIE_DOMAIN` — the public domain of the frontend service
  (e.g. `.your-team.com`).
- `AUTH_COOKIE_SECURE=true`

The backend refuses to start if any of the required `FUSIONAUTH_*` vars
are missing (unless `DISABLE_AUTH=1 + RUST_ENV=development`).

### 4. Admin-only user provisioning

There is no self-signup. To invite a teammate:

1. Visit the FusionAuth admin UI (reachable from inside the Railway
   project — use Railway's "shell" tunnel or temporarily expose it on a
   restricted internal-only domain while creating users).
2. Create the user under the agent-maker application.
3. Assign the `admin` role if they should be able to manage other users.
4. Share the temporary password out-of-band; rotate the kickstart admin's
   password immediately after the first successful production login.

Password resets are also manual (FusionAuth admin UI → user → "Set
password"). Self-serve reset is deferred beyond F10.

### 5. Cutover — breaking change

F10 introduces a login wall and is shipped as a one-time **breaking
change**. Existing pre-F10 deploys must wipe the `agent_maker` database
on cutover so migrations can re-run from scratch and there is no orphan
data.

On Railway: take a backup, then either drop the database from a one-off
shell (`DROP DATABASE agent_maker; CREATE DATABASE agent_maker;`) or
detach + recreate the volume backing the Postgres service. On the next
backend deploy, migrations 0001–0006 reapply against the empty cluster.
Locally, the equivalent is `docker compose down -v && docker compose up
-d`.
