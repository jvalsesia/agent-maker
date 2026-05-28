# F10. Access and Identity — Technical Specification

## 1. Overview

F10 puts the agent-maker workspace behind a login wall. A FusionAuth
instance runs as a private internal service on Railway (and as a
docker-compose service locally), reachable only from the agent-maker
backend over a private network. The browser never talks to FusionAuth
directly: a small React login page (`src/pages/Login.tsx`) posts email
and password to the backend's `/auth/login`, which calls FusionAuth's
`/api/login` server-to-server, exchanges the credentials for an
access + refresh token pair, and writes both to httpOnly secure cookies
on the response. An Axum middleware in front of every non-`/auth/*`,
non-health route validates the access token against a cached JWKS pulled
from FusionAuth at boot, injects an `AuthContext { sub, email, roles }`
into request extensions, and triggers a silent refresh from the
frontend's fetch layer when the access token expires. The workspace is
shared — no `user_id` columns are added to any existing table; the
`admin` role (a single FusionAuth role surfaced as a JWT claim) gates
admin-only actions.

## 2. Scope

**Included:**
- New backend domain `backend/src/auth/` (mod / model / service /
  jwks / cookies) plus `backend/src/routes/auth.rs` exposing
  `/auth/login`, `/auth/logout`, `/auth/refresh`, `/auth/me`.
- Axum middleware `RequireAuth` applied to every non-`/auth/*`,
  non-health route in `build_app`; a thinner `RequireAdmin` layer
  applied to admin-only endpoints.
- FusionAuth client (thin `reqwest` wrapper) for `/api/login`,
  `/api/jwt/refresh`, `/api/logout`, `/.well-known/jwks.json`.
- JWKS cache (in-memory, RS256, refresh every 10 min and on `kid`
  miss).
- Cookie helpers (`access_token`, `refresh_token`): `HttpOnly`,
  `Secure` (configurable for local http), `SameSite=Lax`, `Path=/`,
  `Domain=$AUTH_COOKIE_DOMAIN`.
- Refresh rotation: every successful `/auth/refresh` mints a new
  refresh token via FusionAuth and rewrites both cookies.
- Frontend: `src/pages/Login.tsx`, `src/hooks/useAuth.ts` (TanStack
  Query: `me`, `login`, `logout`), `<RequireAuth>` wrapper component
  in `src/router.tsx`, fetch-layer 401 handler that performs one
  silent refresh before redirecting to login, signed-in email + Sign
  out control in the global nav.
- i18next keys for the login screen and auth error messages in both
  `en.json` and `pt-BR.json`.
- Infra: `docker-compose.yml` gains `fusionauth` and `fusionauth-db`
  services with healthchecks; committed
  `infra/fusionauth/kickstart/kickstart.json` seeds tenant,
  application, API key, RS256 signing key, the `admin` role, and one
  test admin user, with all UUIDs pinned.
- Dev escape hatch: `DISABLE_AUTH=1` (only honored when
  `RUST_ENV=development`) bypasses the middleware and injects a fake
  `AuthContext`.
- Railway deployment notes documenting how to add FusionAuth as a
  private service alongside `backend` + `frontend`.

**Integrated from PRD blocks (F10):**
- `Provides`: authenticated session context + request-gating
  middleware consumed by F02–F09 (every protected route).
- `Capabilities`: FusionAuth private; shared workspace; custom React
  login form proxied through backend; email + password; admin-invite
  only; single `admin` JWT role; access 1h, refresh 30d, rotation on
  use; httpOnly secure cookies; `DISABLE_AUTH` dev flag; kickstart
  for local; breaking-change cutover.
- `Experience`: unauthenticated → `/auth/login` redirect, in-app
  React form, silent refresh, global nav email + "Sign out",
  admin-only UI gated by role claim.
- `Error Handling`: invalid credentials inline message and password
  clear; expired refresh → redirect with "Your session expired";
  FusionAuth unreachable → backend 503; misconfiguration → backend
  refuses to start; 401 on protected call → one silent refresh then
  redirect.

**Deferred (out of F10, per PRD §7 "Identity features deferred"):**
- Self-serve password reset (admins reset manually in FusionAuth
  admin UI).
- Social/SSO, magic link, M2M tokens.
- Self-signup or public registration.
- Audit log.
- Per-user data scoping or per-user secrets — the workspace stays
  shared.

**Assumptions / decisions (PRD did not specify):**
1. **No new tables in the `agent_maker` database.** Shared workspace
   means no `user_id` foreign keys. The pre-F10 database is dropped
   on cutover and recreated from the existing migrations 0001–0006;
   no new SQLx migration is added by F10.
2. **JWT signing algorithm: RS256.** Matches FusionAuth's default;
   the JWKS endpoint exposes the public key; the backend never
   needs the signing secret. HS256 considered and rejected because
   it would force the backend to hold FusionAuth's signing key.
3. **JWKS cache TTL: 10 minutes**, plus an on-miss refresh when a
   token carries an unknown `kid`. Boot-time fetch is mandatory —
   the backend refuses to start if JWKS cannot be fetched.
4. **Cookie names: `am_access` and `am_refresh`.** Prefixed to avoid
   collisions if the same Railway environment ever hosts another
   app.
5. **Access token cookie lifetime matches its JWT exp (1h)**;
   refresh token cookie lifetime is 30d. Both `Max-Age` driven, not
   session cookies, so they survive browser restart.
6. **`/auth/me` is the only authenticated endpoint that does NOT
   silently refresh on 401.** It returns 401 cleanly so the route
   guard can decide whether to attempt a refresh; every other
   protected call goes through the fetch-layer wrapper that handles
   refresh transparently.
7. **Admin role check happens in the backend only.** The frontend
   reads the `admin` flag from `/auth/me` to hide/show UI affordances,
   but the backend is the security boundary; spoofing the role in
   the React tree gains nothing.
8. **CORS stays same-origin.** Frontend is served from the same
   backend in production (existing `serve_frontend_dist` path); local
   dev uses Vite's proxy. No `Access-Control-Allow-Credentials`
   changes needed.

## 3. Component Overview

**Backend (new):**
- `backend/src/auth/mod.rs` — re-exports + `pub use`.
- `backend/src/auth/model.rs` — `AuthContext`, `LoginRequest`,
  `MeResponse`, `Claims` (JWT body), `AuthError` (with `thiserror`).
- `backend/src/auth/service.rs` — `AuthService { login, refresh,
  logout, me }`; depends on `FusionAuthClient` + `JwksCache`.
- `backend/src/auth/jwks.rs` — `JwksCache` (RwLock'd map of
  `kid → DecodingKey`) + background refresher.
- `backend/src/auth/fusionauth.rs` — typed `reqwest` client for
  `/api/login`, `/api/jwt/refresh`, `/api/logout`, JWKS endpoint.
- `backend/src/auth/middleware.rs` — `RequireAuth`, `RequireAdmin`
  Axum `from_fn_with_state` layers.
- `backend/src/auth/cookies.rs` — cookie build/parse helpers.
- `backend/src/routes/auth.rs` — handlers for `/auth/login`,
  `/auth/logout`, `/auth/refresh`, `/auth/me`.

**Backend (modified):**
- `backend/src/lib.rs` — declare `pub mod auth;`, wire
  `AuthService` into `AppState`, attach `RequireAuth` to the protected
  router branch in `build_app`. `/healthz` and `/auth/*` stay public.
- `backend/src/config.rs` — add `FUSIONAUTH_BASE_URL`,
  `FUSIONAUTH_TENANT_ID`, `FUSIONAUTH_APPLICATION_ID`,
  `FUSIONAUTH_CLIENT_ID`, `FUSIONAUTH_CLIENT_SECRET`,
  `FUSIONAUTH_API_KEY`, `AUTH_COOKIE_DOMAIN`,
  `AUTH_COOKIE_SECURE` (bool, default `true`), `DISABLE_AUTH`
  (dev-only). Hard-fail at boot if any required var is missing
  except when `DISABLE_AUTH=1` and `RUST_ENV=development`.
- `backend/Cargo.toml` — add `jsonwebtoken`, `axum-extra` (for
  typed `CookieJar`) if not already present.

**Frontend (new):**
- `frontend/src/pages/Login.tsx` — single-screen email/password
  form, error inline, i18n-aware, posts to `/auth/login`.
- `frontend/src/hooks/useAuth.ts` — TanStack Query hooks: `useMe`,
  `useLogin`, `useLogout`.
- `frontend/src/components/auth/RequireAuth.tsx` — guard component
  that calls `useMe`, renders `<Navigate to="/login" />` on 401.
- `frontend/src/lib/authFetch.ts` — fetch wrapper that performs one
  silent `POST /auth/refresh` on 401 then retries; otherwise
  surfaces the original response.

**Frontend (modified):**
- `frontend/src/router.tsx` — add `/login` route (public); wrap the
  existing protected tree in `<RequireAuth>`.
- `frontend/src/components/Layout.tsx` (or equivalent global nav) —
  show signed-in email; add "Sign out" action calling `useLogout`.
- `frontend/src/lib/api.ts` (or wherever the fetch helper lives) —
  route every existing API call through `authFetch`.
- `frontend/src/i18n/locales/en.json` and `pt-BR.json` — add
  `auth.*` keys: title, fields, submit, errors, signed-in-as,
  sign-out, session-expired.

**Infra (new):**
- `infra/fusionauth/kickstart/kickstart.json` — committed.
- `docs/F10-iam/railway.md` (or update `docs/deployment.md`) — how
  to deploy FusionAuth as a private service on Railway.

**Infra (modified):**
- `docker-compose.yml` — `fusionauth` + `fusionauth-db` services
  with healthchecks; `backend` depends on `fusionauth` health.
- `.env.example` — new `FUSIONAUTH_*`, `AUTH_COOKIE_*`,
  `DISABLE_AUTH` entries with sane local defaults.

## 4. Data Model

No schema changes to the `agent_maker` database in F10. FusionAuth
owns its own Postgres database (`fusionauth-db`), schema managed by
the FusionAuth image — agent-maker does not touch it.

Cutover procedure (documented; not automated):

```bash
docker compose down -v        # drops both postgres volumes locally
docker compose up -d          # fresh DBs; SQLx migrations 0001–0006 re-apply
                              # FusionAuth kickstart re-seeds
```

On Railway the equivalent is to delete the existing `agent_maker`
service database volume (or run a one-off `DROP DATABASE` followed
by service restart so migrations re-apply).

## 5. API Contracts

All `/auth/*` routes are public (no middleware). All others require
a valid `am_access` cookie.

### POST `/auth/login`

Request:
```json
{ "email": "admin@example.com", "password": "…" }
```

Response 200:
```json
{ "email": "admin@example.com", "roles": ["admin"] }
```

Side effects: sets `am_access` (Max-Age 3600) and `am_refresh`
(Max-Age 2592000) cookies. Both `HttpOnly; Secure; SameSite=Lax`.

Errors:
- 401 `{ "error": "invalid_credentials" }` — FusionAuth returns
  any non-success.
- 503 `{ "error": "auth_unavailable" }` — FusionAuth unreachable
  or returned 5xx.

### POST `/auth/refresh`

Request: no body. Reads `am_refresh` cookie.

Response 200:
```json
{ "email": "admin@example.com", "roles": ["admin"] }
```

Side effects: calls FusionAuth `/api/jwt/refresh`, rotates both
cookies on success.

Errors:
- 401 `{ "error": "refresh_invalid" }` — cookie missing, expired,
  or FusionAuth rejects.

### POST `/auth/logout`

Request: no body.

Response 204. Side effects: best-effort call to FusionAuth
`/api/logout`; unconditionally clears both cookies (Max-Age 0).

### GET `/auth/me`

Response 200:
```json
{ "email": "admin@example.com", "roles": ["admin"] }
```

Response 401: cookie missing or token invalid. Does **not** attempt
a silent refresh — the frontend route guard does.

### Middleware contract

Every other route receives an `AuthContext` extension when the
middleware passes:

```rust
pub struct AuthContext {
    pub sub: Uuid,
    pub email: String,
    pub roles: Vec<String>,
}
```

`RequireAdmin` layer asserts `roles.contains(&"admin".to_string())`
or returns 403 `{ "error": "forbidden" }`.

When `DISABLE_AUTH=1` and `RUST_ENV=development`, `RequireAuth`
injects:

```rust
AuthContext {
    sub: Uuid::nil(),
    email: "dev@local".to_string(),
    roles: vec!["admin".to_string()],
}
```

## 6. Error Handling

| Scenario | Surface |
|---|---|
| Invalid credentials at login | 401 `invalid_credentials`; UI shows "Email or password is incorrect", clears password field, focuses email. |
| FusionAuth unreachable at login | 503 `auth_unavailable`; UI shows "Authentication service unavailable — retry shortly". Login button stays enabled. |
| Access token expired | 401 on the protected call → `authFetch` calls `/auth/refresh` once; on success retries the original call transparently. |
| Refresh token expired or revoked | 401 from `/auth/refresh` → `authFetch` clears its in-memory `me` cache, TanStack Query invalidates `me`, `<RequireAuth>` renders `<Navigate to="/login?expired=1" />`; the login page shows "Your session expired". Composer draft (if any) is preserved in component state. |
| FusionAuth unreachable mid-session | `/auth/refresh` 503; UI shows a non-blocking toast "Authentication service unavailable" and keeps the user on the current page until the access token actually expires. |
| Backend misconfigured at boot | Backend logs the missing `FUSIONAUTH_*` env var and exits with a non-zero code — Railway / docker-compose treat this as a failed start. |
| JWKS fetch fails at boot | Same as misconfiguration: backend refuses to start. Mid-session JWKS-refresh failures only log a warning and keep serving with the cached key set until the next attempt. |
| `kid` not in cache | Trigger an on-demand JWKS refresh; if still missing, treat the token as invalid (401). |
| Non-admin hits admin route | 403 `forbidden`. |

## 7. Testing Strategy

**Backend unit tests (in-tree `#[cfg(test)]`):**
- `jwks::tests::decodes_known_kid` — seeds the cache with a fixture
  RSA pubkey; verifies a token signed by the matching privkey
  decodes; an unknown `kid` returns `AuthError::UnknownKid`.
- `cookies::tests::round_trip` — `set_auth_cookies` then parse from
  a `CookieJar` and recover the same values; assert `HttpOnly`,
  `Secure`, `SameSite=Lax`, correct `Max-Age`.
- `middleware::tests::missing_cookie_returns_401`,
  `expired_token_returns_401`, `valid_token_injects_context`,
  `disable_auth_dev_only` (asserts the flag is ignored in non-dev).
- `service::tests::login_maps_fusionauth_401_to_invalid_credentials`
  using a `wiremock` server in front of a fake FusionAuth.

**Backend integration tests (`backend/tests/auth.rs`):**
- `login_happy_path` — uses `compose`'s real FusionAuth (gated on
  `FUSIONAUTH_BASE_URL` env; skipped if absent) to log in with the
  kickstart admin, assert 200 and Set-Cookie headers.
- `protected_route_requires_auth` — `GET /api/agents` without a
  cookie returns 401; with the cookie from the previous test returns
  200.
- `refresh_rotates_tokens` — first refresh succeeds and returns new
  cookies; the old refresh token is rejected on the second attempt.
- `logout_clears_cookies` — Set-Cookie with Max-Age=0 on both
  cookies; subsequent `/auth/me` returns 401.

**Frontend tests (`vitest` + Testing Library):**
- `Login.test.tsx` — happy path POSTs to `/auth/login` and navigates
  to `/`; invalid creds renders the error and clears the password
  field; submit disabled while pending.
- `authFetch.test.ts` — 401 triggers one refresh and retries; second
  401 redirects to `/login?expired=1`; non-401 responses pass
  through untouched.
- `RequireAuth.test.tsx` — renders children when `useMe` returns
  200; renders `<Navigate>` when 401.
- `useAuth.test.ts` — `useLogout` invalidates the `me` query and
  clears TanStack Query cache.

**Acceptance criteria coverage (PRD §9 F10):**
| PRD criterion | Test |
|---|---|
| Unauthenticated → 401 + redirect | `protected_route_requires_auth` + `RequireAuth.test.tsx` |
| Login happy path | `login_happy_path` + `Login.test.tsx` |
| Silent refresh on 1h expiry | `authFetch.test.ts` + `refresh_rotates_tokens` |
| Refresh expiry → "Your session expired" | `authFetch.test.ts` second-401 case |
| Sign out clears cookies + revokes | `logout_clears_cookies` |
| Refresh persists across browser restart | Cookie `Max-Age` covered by `cookies::tests::round_trip`; manual smoke step in `verify.md` |
| Email visible in nav | Layout test |
| Admin role gates admin routes | `RequireAdmin` integration test using a non-admin token |
| Invalid credentials inline | `Login.test.tsx` |
| FusionAuth unreachable → 503 | `login_maps_fusionauth_503_to_auth_unavailable` |
| `DISABLE_AUTH` dev-only | `disable_auth_dev_only` |
| Cutover wipe | Documented; no automated test |
| Docker compose brings up FusionAuth + kickstart | Manual smoke (`verify.md`) |

**Cross-Feature Integration (PRD §9):**
- "Authenticated session from F10 required for every F02–F09
  operation" — covered by `protected_route_requires_auth` extended
  to one route per protected module (`/api/agents`, `/api/skills`,
  `/api/conversations/:id/memory/query`, etc.) via a parameterized
  test.
- "Admin role gates admin-only flows without leaking to non-admins"
  — covered by the `RequireAdmin` integration test.
