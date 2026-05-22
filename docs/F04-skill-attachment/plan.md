# F04. Skill Attachment to Agents — Implementation Plan

## Prerequisites

- F01, F02, F03 are on this branch (`docker compose up -d`, `cargo run`, `pnpm dev`)
- F01's `agent_skills(agent_id, skill_id, position 0..19, UNIQUE(agent_id, position), ON DELETE CASCADE)` table is in place
- F02 ships `agents` rows with `provider` + `model` columns (used by the composed-prompt budget lookup)
- F03 ships `skills` rows with `body` (read fresh by `GET /api/agents/:id/compose`)
- No new migration is required for F04

## Phase 1 — Backend Data and Service Layer

1. **Module scaffold (`backend/src/skill_attachments/`)** — Create `mod.rs`, `model.rs`, `service.rs`, `routes.rs`. Wire `mod skill_attachments;` from `lib.rs` next to `agents` and `skills`. Mirror the F03 module shape so that future readers see a single template across F02/F03/F04.

2. **Domain types (`model.rs`)** — Define `AttachedSkill`, `AttachReplaceInput`, `ComposePreview`, `Warning`, and the `AttachmentError` enum. Derive `serde` + `sqlx::FromRow` where needed. Keep the JSON envelope shape (`{ attached, warnings }`) identical to F03's `{ skill, warnings }` so the frontend's error helper handles both transparently.

3. **Model context map (`agents/models.rs`)** — Add a small `pub fn model_context_chars(provider: &str, model: &str) -> i64` returning the static char budgets documented in spec §2 Assumptions. Unit-test the three provider defaults plus one unknown-model fallback (32 000 × 4).

4. **`skill_attachments::service`** — Implement `list(agent_id)`, `replace(agent_id, skill_ids)`, `detach_one(agent_id, skill_id)`, and `compose(agent_id)`. `replace` runs inside a single `BEGIN/COMMIT` transaction: validate inputs (≤ 20, no duplicates, all `skill_id`s exist), `DELETE FROM agent_skills WHERE agent_id = $1`, then `INSERT … (agent_id, skill_id, position)` with `position = array_index`. `detach_one` deletes one row and renumbers remaining positions in the same TX. `compose` joins the agent's `system_prompt` and the ordered `skills.body` rows with `\n\n` and computes the length + budget warning.

## Phase 2 — Backend REST Surface

5. **`skill_attachments::routes`** — Mount four handlers under `/api/agents/:agent_id/skills*` and `/api/agents/:agent_id/compose` per spec §5. Map `AttachmentError::UnknownSkill` → 422, `AgentNotFound` → 404, `TooManySkills` → 422, `NotAttached` → 404, DB failures → 500 with the F01 envelope. Reuse F03's `Warnings` JSON shape.

6. **Wire routes into the Axum router** — Add `skill_attachments::routes::router()` under the existing `/api` `Router::nest` call in `main.rs`, alongside the F02 `agents` and F03 `skills` routers. Make sure the new routes are matched **before** any `/api/agents/:id` catch-alls that F02 may have added.

7. **Integration tests (`tests/skill_attachments.rs`)** — Cover the lifecycle, all four validation failures, the renumber-after-detach invariant, the cascade on agent/skill deletion, and the `compose` envelope. Use the same disposable-Postgres harness established in F02/F03.

## Phase 3 — Frontend

8. **REST client (`lib/agentSkills.ts`)** — Strongly typed wrappers (`listAttached`, `replaceAttached`, `detachOne`, `getCompose`). All functions throw the same typed `ApiError` used by F02/F03 so components can branch on `code`.

9. **React Query hooks (`useAgentSkills.ts`)** — Provide `useAttachedSkills(agentId)`, `useComposePreview(agentId)`, and mutation hooks for replace + detach. Successful mutations invalidate `['agent-skills', agentId]`, `['agent-compose', agentId]`, and `['skills']` (so the F03 list's badge refreshes). Mutations are optimistic with rollback on error.

10. **`AgentSkillsPanel.tsx`** — New section inside the F02 `AgentForm` (edit mode only). Renders the ordered attached skills with drag handles (`@dnd-kit/core`) and a per-row detach button. Add up/down keyboard shortcuts on the focused row for accessibility. Below the list, render `ComposedPromptIndicator`.

11. **`AttachSkillsDialog.tsx`** — Multi-select modal listing every workspace skill with name, description, and a checkbox. Already-attached skills are pre-checked and visually muted. Search filter is client-side (case-insensitive contains on name + description). Confirm assembles the merged ordered list and fires the replace mutation.

12. **`ComposedPromptIndicator.tsx`** — Reads `useComposePreview(agentId)` and renders neutral / yellow / red states at 0–79 % / 80–94 % / ≥ 95 % of the model budget. Uses the same shadcn `Alert` primitive F02/F03 already wire.

## Phase 4 — Verification

13. **Frontend tests** — Implement the Vitest + Testing Library cases enumerated in spec §9 under `frontend/src/pages/Agents/`. Reuse the MSW-style mock-fetch handler conventions established in F02/F03; add `/api/agents/:id/skills*` and `/api/agents/:id/compose` handlers.

14. **Manual end-to-end smoke** — With the full stack running: create two skills via F03 → open an agent in F02 → attach both via the modal (verify pre-check on reopen) → drag-reorder → verify order persists on reload → detach one (verify no confirmation modal, immediate removal) → bump a skill body to be very large → verify the indicator turns yellow then red → delete the agent and confirm `SELECT count(*) FROM agent_skills WHERE agent_id = ...` returns 0 (cascade). Capture the steps in `docs/F04-skill-attachment/verify.md`.
