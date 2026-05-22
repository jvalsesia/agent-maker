# F02 — Verification

## Automated

### Backend

```bash
docker compose up -d
cd backend
DATABASE_URL="postgres://agentmaker:agentmaker@127.0.0.1:5432/agentmaker" cargo test
```

Expected counts (current baseline):

- `cargo test --lib` → 17 passed (includes 10 new in `agents::service::tests`)
- `cargo test --test agents` → 10 passed
- `cargo test --test integration` → 10 passed

The `tests/agents.rs` suite exercises the full HTTP lifecycle against a real Postgres
provisioned per test by `sqlx::test`:

- create → list → get golden path
- duplicate name returns a field error
- `provider: "bogus"` returns 400 with `field: "provider"`
- `PUT` bumps `updated_at` (validates the F02 trigger from `0003_agents_touch.sql`)
- clone suffixes ` (copy)` then ` (copy) (2)` on collision
- agent delete removes the namespaced `agent:<id>:<provider>` secret AND the row
- per-agent key save returns the masked form and flips `has_override_key`; delete clears it
- list `?sort=name&order=asc` and `?q=eta` return the expected rows
- `GET /api/agents/models?provider=anthropic` returns a non-empty list
- soft warnings surface in the response body for too-short prompts

### Frontend

```bash
cd frontend
pnpm typecheck && pnpm test --run && pnpm build
```

Expected: typecheck clean, 11 tests pass (4 new under `src/pages/Agents/`), and the
production build succeeds.

## Manual smoke (golden path)

Prereqs: stack running (`docker compose up -d`, `cargo run`, `pnpm dev`), at least one
provider key configured in Settings.

1. **Empty state** — Open http://localhost:5173/agents. The empty-state card
   appears with the "Create agent" CTA. Click it.
2. **Create** — Fill in name `Editor`, a short preamble, a system prompt of ~100
   characters, leave Provider as Anthropic, pick a model. Click **Save**. A toast
   confirms; the URL updates to `/agents/<id>`; the form is hydrated with the new
   agent.
3. **Reload persistence** — Reload the page. The agent's fields are still populated.
4. **Soft warning** — Replace the system prompt with `hi`. Save. A muted amber
   warning appears under the textarea explaining the prompt is shorter than 50 chars.
5. **Hard validation** — Try to set the name to an empty string and save. The
   backend returns 400; an inline error renders under the name input.
6. **Duplicate name** — From the list, click **+ New Agent**, enter `Editor`,
   fill the rest, save. The inline error under "Name" reads `an agent with this name
   already exists`.
7. **Clone** — In the list, click the copy icon on the `Editor` row. A toast says
   `Cloned as Editor (copy)`. Cloning again produces `Editor (copy) (2)`.
8. **Per-agent key** — Open `Editor`, expand **Advanced**, paste a key, click
   **Save key**. The toast surfaces the masked form. Reload; the section still
   reports `An override key is currently set for this agent.`. Click **Remove
   override**; the message disappears.
9. **No-key warning** — Change the provider to one that has no key in Settings
   (e.g. OpenAI without a configured key). The amber warning appears under the
   provider dropdown with a "Go to Settings" link.
10. **Sort & search** — Back on `/agents`, type `cop` into search; only the
    cloned rows remain. Change sort to "Created" to verify the order changes.
11. **Delete** — Click the trash icon on `Editor (copy) (2)`. The dialog lists
    `0 conversation(s)` and `0 attached skill(s)`. Confirm. The row disappears.
12. **Direct URL guard** — Visit `/agents/00000000-0000-0000-0000-000000000000`.
    The backend returns 404; the form renders the fallback title. No crash.

## Cross-feature placeholders

- `attached_skill_count` and `conversation_count` are always `0` until F04 and F06
  ship. F02's list query already joins to both tables — the counts will populate
  automatically once those features insert rows.
- "Save and chat" navigates to the agent detail page. F06 will repurpose this
  action to open a fresh conversation route.
