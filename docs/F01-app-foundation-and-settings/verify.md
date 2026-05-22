# F01 — Manual Verification Checklist

A short regression script you can walk in 5–10 minutes whenever F01 (foundation, settings, key storage) is touched. Pair it with the automated suites (`cargo test`, `pnpm test`).

## Prerequisites

- Docker running
- Rust toolchain (`cargo --version`)
- Node + pnpm (`pnpm --version`)
- A clean state — if `~/.agent-maker/` exists from a prior run and you want a fresh test, remove it first

## 1. Bring the stack up

```bash
# from repo root
docker compose up -d                                  # Postgres + pgvector on 127.0.0.1:5432
cd backend && cargo run                               # in terminal A — backend on 127.0.0.1:8787
cd frontend && pnpm dev                               # in terminal B — Vite dev server on 5173
```

Expected backend logs:
- `database reachable`
- `database migrations applied`
- `secret store: keyring` (or `OS keychain unavailable, falling back to file store`)
- `listening addr=127.0.0.1:8787`

## 2. Health endpoint

```bash
curl -s http://127.0.0.1:8787/api/health | jq
# {"status":"ok","db":"ok","key_store":"keyring"}
```

## 3. Onboarding (first-launch redirect)

- Open `http://localhost:5173/`.
- With zero providers configured, the router should send you to `/onboarding`.
- The "Welcome to agent-maker" card lists the three providers and a single API-key field.

## 4. Save a key + test connection

- Pick a provider (e.g. Anthropic). Paste a real key. Click **Save and continue**.
- You land on `/settings` → **Providers** tab.
- The masked key is shown (e.g. `sk-ant-***...XXXX`). The eye toggle shows/hides the *mask*, never a raw key.
- Click **Test connection** → the inline message reads `OK — <model> (<latency> ms)`.
- Replace the key with a deliberately invalid one and re-test → expect a provider-specific `401 invalid x-api-key` style message.

## 5. Memory Defaults

- Switch to **Memory Defaults** tab.
- Move `recent_n` to 12, `top_k` to 6 → save → toast `Settings saved`.
- Reload the page → values persist.
- Try setting `recent_n=2` → the request is rejected with an inline `validation_error` message and the slider/input snaps back.

## 6. Appearance (theme switch)

- Open **Appearance** tab.
- Toggle `light` / `dark` / `system`; the `<html>` element gains/loses the `dark` class accordingly.
- Reload — the theme persists.

## 7. Data wipe

- Open **Data** tab → **Wipe local data**.
- Cancel the modal first → nothing happens.
- Re-open, type `WIPE`, confirm.
- After the toast, navigate to **Providers** → all keys show `not configured`.
- The app should redirect back to `/onboarding` on the next reload.

## 8. "Database unavailable" failure mode

```bash
docker stop agent-maker-postgres
```

- Reload the SPA. The bootstrap call to `/api/health` will fail and the app should show the full-screen "Database unavailable" message with the DSN and the `docker compose up -d` hint.
- Bring it back: `docker start agent-maker-postgres`. Wait ~2s for the healthcheck. Reload — the app recovers.

## 9. Secret store backend

agent-maker has two secret stores (OS keyring + AES-256-GCM file fallback).
See [`secret-store.md`](./secret-store.md) for the full picture; quick checks:

- `GET /api/settings` reports the active backend at `key_store_backend`.
- On Linux desktops where Secret Service is unreliable, set
  `AGENT_MAKER_FORCE_FILE_STORE=1` in `.env` to force the file store. Boot log
  shows `secret store: file (forced via AGENT_MAKER_FORCE_FILE_STORE)`.
- When file store is active, the Providers section shows the banner _"Using
  encrypted file fallback — OS keychain unavailable."_ and
  `ls -l ~/.agent-maker/` shows `master.key` and `secrets.bin` with mode `0600`.
- After switching backends, run the wipe + re-save flow from
  [`secret-store.md` → Switching backends](./secret-store.md#switching-backends)
  or you'll see "configured but `no_key`" errors.

## 10. Automated suites (sanity)

```bash
# backend (requires docker compose up -d)
cd backend && DATABASE_URL=postgres://agentmaker:agentmaker@127.0.0.1:5432/agentmaker cargo test

# frontend
cd frontend && pnpm test
```

Expected: backend → **17 passed** (7 unit + 10 integration), frontend → **7 passed**.
