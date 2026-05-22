# F05 — Manual Verification Smoke

Run end-to-end against a clean workspace to confirm the spec §10 acceptance
criteria. This complements the automated suites:

- Backend: `cargo test --test templates` (10 tests, real Postgres pool)
- Frontend: `npx vitest run src/pages/Templates` (5 tests, MSW-style mock fetch)

## Setup

```bash
docker compose up -d                     # Postgres + pgvector
cargo run -p agent-maker                 # backend at :8080
pnpm --filter frontend dev               # frontend at :5173
```

Wipe any prior state from Settings → Data → Wipe so the workspace starts empty.

## Steps

1. **Open `/templates`** — sidebar "Templates" item is enabled; gallery renders
   **20 cards** (10 agents + 10 skills).
2. **Filter by Writing chip** — only Writing-category agents and skills remain.
3. **Type `editor` in the search box** — list narrows to `Writing Editor` (and the
   `Blunt Editor` skill).
4. **Click Preview on `Writing Editor`** — drawer opens; system-prompt body is
   visible in a monospace block; `Blunt Editor` and `Concise Replies` appear in
   the suggested-skills list.
5. **Click Adopt in the drawer** — success toast reads
   `Adopted "Writing Editor"` with an inline `Open agent` action.
6. **Click `Open agent`** — lands on `/agents/<uuid>`; the attached-skills panel
   shows `Blunt Editor` at position 1 and `Concise Replies` at position 2.
7. **Return to `/templates` → adopt `Writing Editor` again** — toast reads
   `Adopted "Writing Editor (template)"`. `/agents` now lists two agents.
8. **Adopt a skill directly** — click Adopt on the `Concise Replies` card → toast
   with `Open skill` shortcut → `/skills` lists `Concise Replies (template)`
   (since step 5 already created one).
9. **Filter combo with no matches** — set Wellbeing chip + search `xyz` → empty
   state copy `No templates match these filters.` with `Clear filters` link.
10. **Clear filters** — gallery returns to all 20 cards.

## Recovery

- If adopt fails (kill Postgres mid-call): toast surfaces the error; no rows are
  created (single TX rollback covered by `adopt_agent_unknown_slug_rolls_back`).
- Unknown slug routes return 404 `template_not_found` — verified by integration
  test `unknown_slug_returns_template_not_found`.
