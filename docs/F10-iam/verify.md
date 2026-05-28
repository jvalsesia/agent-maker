# F10 — Manual verification

End-to-end smoke checks for the FusionAuth-backed login flow. Run
these after `docker compose up -d` and a backend rebuild with
`DISABLE_AUTH=0` (or unset).

## Setup

1. Confirm FusionAuth is up:

   ```bash
   docker compose ps fusionauth fusionauth-db
   # expect both Healthy
   curl -fsS http://127.0.0.1:9011/api/status >/dev/null && echo OK
   ```

   Boot takes 30–60 s the first time. Watch logs with
   `docker compose logs -f fusionauth` until you see
   `agent-maker tenant ... configured` (kickstart finished).

2. Confirm backend talks to FusionAuth:

   ```bash
   docker compose logs backend | grep -i "jwks\|fusionauth"
   # expect no "initial JWKS fetch failed" lines after FusionAuth is up
   ```

3. Make sure `.env`/compose has:
   - `DISABLE_AUTH=` (empty) or `RUST_ENV=production`
   - `AUTH_COOKIE_SECURE=false` (local http) — flip to `true` over https

## 1. Anonymous request is rejected

```bash
curl -i http://127.0.0.1:8787/api/agents
# expect HTTP/1.1 401 Unauthorized
# body: {"error":"missing_token"}
```

## 2. Health endpoint stays public

```bash
curl -fsS http://127.0.0.1:8787/api/health
# expect 200 + {"status":"ok", ...}
```

## 3. Login happy path

Use the kickstart admin credentials:

```bash
curl -i -c /tmp/am-cookies -X POST http://127.0.0.1:8787/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@agent-maker.local","password":"ChangeMeNow!1"}'
# expect 200 + {"email":"admin@agent-maker.local","roles":["admin"]}
# expect Set-Cookie: am_access=...; HttpOnly; SameSite=Lax
# expect Set-Cookie: am_refresh=...; HttpOnly; SameSite=Lax
```

## 4. Authenticated request succeeds

```bash
curl -fsS -b /tmp/am-cookies http://127.0.0.1:8787/api/agents
# expect 200 + {"agents":[...]}

curl -fsS -b /tmp/am-cookies http://127.0.0.1:8787/auth/me
# expect 200 + {"email":"admin@agent-maker.local","roles":["admin"]}
```

## 5. Refresh rotates tokens

```bash
curl -i -b /tmp/am-cookies -c /tmp/am-cookies-2 \
  -X POST http://127.0.0.1:8787/auth/refresh
# expect 200 + fresh am_access and am_refresh Set-Cookie headers
```

## 6. Logout clears cookies

```bash
curl -i -b /tmp/am-cookies -X POST http://127.0.0.1:8787/auth/logout
# expect 204
# expect Set-Cookie: am_access=; Max-Age=0
# expect Set-Cookie: am_refresh=; Max-Age=0
```

After logout, /api/agents with the cleared cookie jar should 401.

## 7. Browser walk-through (UI)

1. Open `http://127.0.0.1:8080`.
2. Confirm immediate redirect to `/login`.
3. Enter the kickstart credentials, click "Sign in".
4. Land on the agents list. Confirm the email appears at the bottom of
   the left nav with a "Sign out" action above it.
5. Close the browser. Reopen and navigate back to the app — should
   *not* prompt for password (refresh-token cookie persists).
6. Click "Sign out". Confirm redirect to `/login`. Refreshing protected
   routes should land on `/login`.
7. Force expiry by trimming the `am_access` cookie in devtools, then
   visiting any protected page. The fetch-layer should call
   `/auth/refresh` once silently; the page should render without a
   visible interruption.
8. Force full session expiry by clearing both cookies in devtools.
   Visit a protected page — should redirect to `/login?expired=1`
   showing "Your session expired."

## 8. Invalid credentials feedback

On the login form, submit a wrong password. Confirm:

- Inline error "Email or password is incorrect."
- Password field is cleared and focused.
- Email field retains its value.

## 9. Admin role gating

`/auth/me` for the kickstart admin returns `roles: ["admin"]`. There
are no admin-only frontend routes in F10, so this is a backend-only
check; the `require_admin` middleware is wired and ready for future
use.

## 10. FusionAuth unreachable

```bash
docker compose stop fusionauth
curl -i -c /tmp/am-cookies -X POST http://127.0.0.1:8787/auth/login \
  -H 'content-type: application/json' \
  -d '{"email":"admin@agent-maker.local","password":"ChangeMeNow!1"}'
# expect 503 + {"error":"auth_unavailable"}
docker compose start fusionauth
```

## 11. DISABLE_AUTH dev escape hatch

Restart the backend with `RUST_ENV=development DISABLE_AUTH=1` (the
default in `.env.example`). Then:

```bash
curl -fsS http://127.0.0.1:8787/auth/me
# expect 200 + {"email":"dev@local","roles":["admin"]}

curl -fsS http://127.0.0.1:8787/api/agents
# expect 200 (no cookies needed)
```

Flip `RUST_ENV=production` with `DISABLE_AUTH=1` still set — the flag
must be **ignored** and `/api/agents` returns 401 without cookies.
