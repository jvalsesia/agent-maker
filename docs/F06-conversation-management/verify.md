# F06 — Manual Verification Smoke

Run end-to-end against a workspace with at least one agent to confirm the
spec §9 acceptance criteria. Complements the automated suites:

- Backend: `cargo test --test conversations` (9 tests, real Postgres pool)
- Frontend: `npx vitest run src/pages/Chat` (5 tests, mock-fetch handler)

## Setup

```bash
docker compose up -d                     # Postgres + pgvector
cargo run -p agent-maker                 # backend at :8080
pnpm --filter frontend dev               # frontend at :5173
```

If the workspace has no agents yet, create one (e.g., adopt `Writing Editor`
from `/templates`) so the chat surface has somewhere to land.

## Steps

1. **Auto-create on first open** — From `/agents`, click the chat icon on any
   row → lands on `/agents/:id/chat` → sidebar shows exactly one entry
   titled `New conversation`.
2. **Multi-create** — Click `+ New conversation` twice → three entries; the
   newly created ones appear at the top (last-activity DESC).
3. **Draft preservation across switches** — Type `alpha draft` in the
   composer placeholder → click another conversation row → type
   `beta draft` → click the first conversation again → composer shows
   `alpha draft`.
4. **Inline rename persists** — Hover the active row → click the pencil
   icon → type `Renamed thread` → press Enter → reload the page → the
   renamed title is still there.
5. **Delete confirmation with message count** — Hover a row → click the
   trash icon → confirmation reads `... 0 messages ...` (or whatever the
   row showed); click Delete → row disappears; the next conversation in
   the sorted list becomes active.
6. **Manual message hydration** — Open psql:
   ```sql
   INSERT INTO messages (conversation_id, role, content)
   VALUES ('<active conv uuid>', 'user', 'hello'),
          ('<active conv uuid>', 'assistant', 'hi back');
   ```
   Reload `/agents/:id/chat` → the two bubbles appear in order; the empty-
   state placeholder is gone.
7. **Validation rejection** — Try to rename a conversation to an empty
   string (clear the input, press Enter) → the inline edit closes without
   persisting (client-side fast-path); attempt a 100-char title via the API
   directly to confirm the backend 400.
8. **Open chat from agent edit form** — Navigate to `/agents/:id` → click
   `Open chat` in the header → lands on the same `/agents/:id/chat`
   surface.

## Recovery

- If a rename returns 500 (kill Postgres mid-call): toast surfaces the
  error; sidebar reverts the displayed title.
- Deleting an active conversation auto-selects the next one in the sorted
  list; if the list becomes empty, the next list-fetch auto-creates a fresh
  singleton (handled by the GET endpoint).
- Hard refresh clears drafts (in-session only by design per spec §6).
