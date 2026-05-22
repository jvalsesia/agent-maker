# F03. Skill Management — Manual Smoke Checklist

Run against the full stack locally (`docker compose up -d` → `cargo run` → `pnpm dev`).
At least one provider key must be configured (so the Gate lets the app past onboarding).

## Steps

1. **Create** — Open `/skills/new`. Enter name `Concise`, description `Keep replies tight`, body (≥ 50 chars). Save. Toast `Skill saved`. The URL becomes `/skills/:id`.
2. **Persist across reload** — Reload the page. The form rehydrates with the saved values.
3. **List** — Navigate to `/skills`. The new skill appears with the description and a `0 agents` badge.
4. **Duplicate name** — `+ New Skill`, enter the same name `Concise`, try to save. Inline error under "Name": "a skill with this name already exists".
5. **Edit** — Open the original skill, change the body. Save. Toast `Skill saved`. Reload — body is updated.
6. **Clone** — From the detail page, click `Clone`. Lands on the new skill `Concise (copy)`. List now shows both.
7. **Manual attachment fixture** — In another terminal:
   ```sh
   psql "$DATABASE_URL" -c \
     "INSERT INTO agent_skills (agent_id, skill_id, position)
      VALUES ((SELECT id FROM agents LIMIT 1), (SELECT id FROM skills WHERE name='Concise'), 0);"
   ```
   (Requires at least one agent in the workspace.)
8. **Using-agents panel** — Reload `/skills/:id`. Right-side panel reads `Used by 1 agent` and lists the agent's name with a link to `/agents/:id`.
9. **List badge updates** — Back on `/skills`, the badge for `Concise` reads `1 agent`.
10. **Sort by attached** — Switch the sort dropdown to `Attached agents`. `Concise` floats to the top.
11. **Delete modal lists agents** — Click the trash icon on `Concise`. The modal lists the using agent by name. Click anywhere on the dimmed background — the modal does NOT dismiss (PRD requirement). Press Esc — modal dismisses.
12. **Delete cascade** — Re-open the delete modal, click `Delete`. Toast `Deleted Concise`. The skill disappears from the list. In psql, `SELECT count(*) FROM agent_skills WHERE skill_id=...` returns `0` (cascade).
13. **Delete unattached skill** — Delete `Concise (copy)`. Modal body reads "No agents currently use this skill". Confirm. Gone.
