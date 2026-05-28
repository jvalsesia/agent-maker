# F10. Access and Identity — Implementation Plan

## Prerequisites

- F01–F09 are on `main`; `docker compose up -d` brings up the existing
  `postgres` service and the backend serves on `127.0.0.1:8787`.
- This is a breaking change. Before starting, expect to run
  `docker compose down -v` once during cutover so the `agent_maker`
  database is wiped and migrations 0001–0006 reapply against the
  empty cluster (the PRD documents this as the one-time F10 cutover).
- FusionAuth's container image will add ~600 MB of memory pressure
  and 30–60s of boot time to the local dev stack; the `DISABLE_AUTH`
  escape hatch lands in Phase 1 so inner-loop work doesn't pay it.

## Phase 1 — Infra and configuration

1. **docker-compose services** — Add `fusionauth` (image
   `fusionauth/fusionauth-app:latest`) and `fusionauth-db` (Postgres
   16, separate volume from `agent_maker`'s Postgres) with
   healthchecks. Update the `backend` service to `depends_on:
   fusionauth: { condition: service_healthy }`. Expose FusionAuth on
   `127.0.0.1:9011` for local admin access only.

2. **Kickstart file** — Commit
   `infra/fusionauth/kickstart/kickstart.json` with pinned UUIDs for
   the tenant, application, API key, RS256 signing key, the `admin`
   role, and one test admin user (credentials documented in
   `.env.example`). Mount it into the `fusionauth` container per the
   image's `FUSIONAUTH_APP_KICKSTART_FILE` convention.

3. **Config + env** — Extend `backend/src/config.rs` with the new
   `FUSIONAUTH_*`, `AUTH_COOKIE_*`, `DISABLE_AUTH`, and `RUST_ENV`
   variables; refuse to start when required vars are missing unless
   the dev escape hatch is active. Mirror the additions in
   `.env.example` with local defaults that match the kickstart.

## Phase 2 — Backend auth module and routes

4. **Module scaffold (`backend/src/auth/`)** — Create `mod.rs`,
   `model.rs`, `service.rs`, `jwks.rs`, `fusionauth.rs`,
   `middleware.rs`, `cookies.rs`. Wire `pub mod auth;` from
   `lib.rs` next to the existing domains. Define `AuthContext`,
   `Claims`, `AuthError`, and the request/response DTOs per spec §3.

5. **FusionAuth client (`fusionauth.rs`)** — Typed `reqwest` wrapper
   for `/api/login`, `/api/jwt/refresh`, `/api/logout`, and the
   JWKS endpoint. Reads base URL + API key from config; surfaces
   FusionAuth 4xx as `AuthError::InvalidCredentials` and 5xx /
   network as `AuthError::Unavailable`.

6. **JWKS cache (`jwks.rs`)** — RwLock'd `kid → DecodingKey` map.
   Fetches once at boot (mandatory — backend exits on failure),
   refreshes every 10 min from a background Tokio task, and refreshes
   on demand when a token carries an unknown `kid`.

7. **Cookie helpers (`cookies.rs`)** — `set_auth_cookies`,
   `clear_auth_cookies`, `extract_access`, `extract_refresh`. All
   cookies `HttpOnly; Secure (config-gated); SameSite=Lax; Path=/`
   with `Max-Age` from spec §2 assumptions.

8. **`AuthService` (`service.rs`)** — Composes the FusionAuth client
   and the JWKS cache. Methods `login`, `refresh`, `logout`, `me`
   map directly to the route handlers; `validate_access` is reused
   by the middleware.

9. **Middleware (`middleware.rs`)** — `RequireAuth` Axum layer that
   extracts `am_access`, validates against JWKS, injects
   `AuthContext`, and short-circuits with 401 on failure.
   `RequireAdmin` is a second thin layer asserting the `admin`
   role. Honors `DISABLE_AUTH` only when `RUST_ENV=development`.

10. **Routes (`routes/auth.rs`)** — Four handlers per spec §5,
    wired into the public `/auth` sub-router. Wire `RequireAuth`
    around the existing protected router branch in `build_app`;
    `/healthz` and `/auth/*` stay outside it.

## Phase 3 — Frontend auth surface

11. **`useAuth` hook + `authFetch`** — `src/hooks/useAuth.ts` exposes
    `useMe`, `useLogin`, `useLogout` over TanStack Query.
    `src/lib/authFetch.ts` wraps `fetch` to perform one silent
    `POST /auth/refresh` on 401 then retry; otherwise pass the
    response through. Route every existing API helper through it.

12. **Login page** — `src/pages/Login.tsx`: email + password form
    styled with the existing UI primitives, i18n-aware, inline
    error surface for `invalid_credentials` (clears password +
    refocuses email) and `auth_unavailable`. On success, navigates
    to the originally requested path or `/`.

13. **Route guard + global nav** — `<RequireAuth>` component in
    `src/components/auth/`, applied at the protected sub-tree root
    in `src/router.tsx`. Add the signed-in email and "Sign out"
    action to the existing layout / global nav.

14. **i18next strings** — Add an `auth.*` namespace to both
    `en.json` and `pt-BR.json` covering title, fields, submit,
    every error message, "Signed in as", "Sign out", and "Your
    session expired".

## Phase 4 — Verification, docs, and cutover

15. **Backend tests** — Land the unit tests for `jwks`, `cookies`,
    `middleware`, and `service` per spec §7, plus the
    `backend/tests/auth.rs` integration suite that talks to the
    docker-compose FusionAuth using the kickstart admin.

16. **Frontend tests** — `Login.test.tsx`, `authFetch.test.ts`,
    `RequireAuth.test.tsx`, `useAuth.test.ts` per spec §7. Mock
    the backend with the existing test harness conventions.

17. **Railway deployment notes** — Update `docs/deployment.md` (or
    add `docs/F10-iam/railway.md`) with: how to add FusionAuth as
    a private internal service, the env vars the `backend` service
    must reference (`FUSIONAUTH_BASE_URL=http://fusionauth.railway.internal:9011`
    etc.), how to run the kickstart on first deploy (mount as
    `FUSIONAUTH_APP_KICKSTART_FILE` or invoke the admin API once),
    and the cutover steps for wiping the existing `agent_maker`
    database.

18. **Manual smoke (`verify.md`)** — Document the end-to-end smoke
    test: `docker compose up -d`, wait for FusionAuth health, log in
    with the kickstart admin, hit a protected route, close the
    browser, reopen, confirm still signed in, sign out, confirm
    redirect to login.
