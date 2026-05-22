# F04. Skill Attachment to Agents — Manual Smoke Checklist

Run against the full stack locally (`docker compose up -d` → `cargo run` → `pnpm dev`).
At least one provider key must be configured (F01 onboarding gate).
At least one agent (F02) and two skills (F03) should be available — create them via the UI before starting.

## Steps

1. **Open agent detail** — Navigate to `/agents/:id` for an existing agent. Confirm a new "Skills" section is rendered between the System prompt textarea and the Advanced panel, showing "No skills attached. Click 'Attach skills' to add behavior to this agent." The "Composed prompt: N / 800,000 chars (0%)" footer is visible.

2. **Attach two skills** — Click `Attach skills`. The modal lists every workspace skill with name + description. Check two skills and click `Attach selected`. Toast `Skills updated`. Modal closes. The Skills section shows both rows in the order they were added (newly checked items appended in alphabetical name order). Composed-prompt counter updates upward.

3. **Persist across reload** — Reload the page. The two attached skills rehydrate in the same order.

4. **Reorder** — Click the down-arrow on the first row. The row swaps with the next one. Click up-arrow on the same row → it swaps back. Tab to the arrow with the keyboard and press Enter — same effect (keyboard accessibility per PRD).

5. **Reorder persists** — Move the second row to the top, then reload. The new order persists.

6. **Detach** — Click the `⨯` on one row. The row disappears **without** a confirmation modal (PRD requirement). The remaining row(s) renumber down to start at position 0 (verify with `psql "$DATABASE_URL" -c "SELECT skill_id, position FROM agent_skills WHERE agent_id='<id>' ORDER BY position;"`).

7. **Attach modal pre-checks existing** — Open the modal again. The currently-attached skill is pre-checked and visually muted. Uncheck it and click `Attach selected` → it detaches.

8. **20-skill cap** — Create more skills if needed, then attempt to attach 21 by manually `curl`ing the PUT endpoint:
   ```sh
   curl -X PUT "http://localhost:3000/api/agents/<id>/skills" \
     -H 'content-type: application/json' \
     -d "$(jq -nc --argjson ids "$(psql -tA "$DATABASE_URL" -c 'SELECT json_agg(id) FROM (SELECT id FROM skills LIMIT 21) x;')" '{skill_ids: $ids}')"
   ```
   Expect `400` with `error.code = "validation_error"` and `error.field = "skill_ids"`.

9. **Composed warning — yellow** — Edit one attached skill (via F03) and paste a body close to 5,000 characters. Open the agent. The footer turns yellow with "composed prompt is at NN% of model budget…" once the fraction crosses 80 %.

10. **Composed warning — red** — Push the body further (or attach several large skills) until the fraction crosses 95 %. The footer turns red with "near model context limit — chat may fail". Chat is still allowed (per PRD); F04 never blocks the save.

11. **Skill-body edit propagates** — While the agent detail page is open, change a skill's body in another tab. Reload the agent page. The composed-prompt counter reflects the new length (F04 reads `skills.body` fresh on every compose query — no caching).

12. **Cascade on agent delete** — Delete the agent. Confirm `SELECT count(*) FROM agent_skills WHERE agent_id='<id>'` returns `0` (FK cascade from F01).

13. **Cascade on skill delete** — Re-create an agent + skill + attachment. Delete the skill via F03. Confirm the `agent_skills` row is gone and the F03 delete modal had listed the agent in its "using agents" panel before deletion.

## Deviations from spec

- **Reorder UX:** spec §3 proposed `@dnd-kit/core` drag-and-drop; this phase ships accessible up/down arrow buttons instead (focusable, keyboard-activatable). Still satisfies PRD §6 F04 ("keyboard shortcuts for accessibility"). Drag-and-drop can be added later without backend changes.
- **`model_context_chars` location:** spec §2 placed it in `backend/src/agents/models.rs`; it lives in `backend/src/skill_attachments/service.rs` next to its only caller. One-line move if F07 wants it elsewhere.
