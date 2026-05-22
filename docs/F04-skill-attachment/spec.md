# F04. Skill Attachment to Agents — Technical Specification

## 1. Overview

F04 wires the many-to-many relationship between agents (F02) and skills (F03) by giving each agent an **ordered** list of attached skill bodies. F01 already created the `agent_skills(agent_id, skill_id, position)` table with `position BETWEEN 0 AND 19`, `UNIQUE(agent_id, position)`, and `ON DELETE CASCADE` from both sides — F04 ships the REST surface and the UI to manage that table from the agent detail view. The composed-prompt length (system prompt + ordered skill bodies) is computed server-side and surfaced in the UI with a warning when it approaches the agent's selected model context limit, satisfying PRD §6 F04 Error Handling. F04 does **not** dispatch any LLM request — the composed prompt is only previewed; F07 will consume it at chat time.

## 2. Scope

**Included:**
- REST endpoints under `/api/agents/:agent_id/skills/*` for list, replace (atomic attach + detach + reorder), and detach
- A second, read-only endpoint `/api/agents/:agent_id/compose` returning the server-composed prompt preview, its length in characters, and the model context budget in characters
- Frontend: a "Skills" section inside the agent detail page (built in F03's `AgentForm`) with the currently attached skills in order, drag handles, up/down keyboard shortcuts, and a one-click detach button per row
- "Attach skills" modal: full skill list with name + description, a search box, multi-select checkboxes, "Attach selected" CTA
- Composed-prompt length indicator (chars and % of model budget) with a warning chip when ≥ 80 % of budget
- Reuse F02/F03 conventions: JSON error envelope, soft-validation `warnings` array, shadcn primitives, sonner toasts, TanStack Query invalidation across `['agents', id]` and `['skills']`

**Integrated from PRD blocks:**
- `Capabilities`: 0–20 attached skills per agent (DB enforces `position 0..19`); drag-and-drop reorder + keyboard up/down; multi-select picker; ordered concatenation into the composed prompt
- `Experience`: "Skills" section on agent detail, "Attach skills" modal, one-click detach (no confirmation), composed-length warning visible inline
- `Error Handling`: composed-prompt-too-large warning surfaces inline; attach/detach failures revert UI and show a toast with retry

**Deferred (out of F04):**
- Actual LLM dispatch of the composed prompt (F07)
- The "Earlier relevant context" memory block (F08) — composed-prompt length does not budget it; F07 owns dynamic K-trimming
- Provider/model context-limit registry beyond a small static map (extended in F07)

**Assumptions / decisions (PRD did not specify):**
1. **Replace-and-reorder atomic write.** Instead of three endpoints (attach / detach / reorder), F04 ships one `PUT /api/agents/:id/skills` that takes the ordered list of `skill_ids` and rewrites `agent_skills` for that agent in a single transaction. This keeps the position invariant `UNIQUE(agent_id, position)` trivially satisfied (we delete then insert under the same TX) and gives drag-and-drop reorder a clean primitive. A small `DELETE /api/agents/:id/skills/:skill_id` endpoint exists only so the one-click detach button does not have to send the full list.
2. **Model context budget.** A static `model_context_chars` map lives in `backend/src/agents/models.rs` (added in F02, extended here for F04's preview). Defaults: Anthropic Claude family → 200 000 tokens × 4 chars; OpenAI GPT-4o family → 128 000 × 4; `openai_compat` (Ollama/LM Studio) → 32 000 × 4 unless the model name carries a recognized suffix. The conversion factor 4 chars/token is a deliberate conservative approximation — F07 will replace it with a tokenizer.
3. **Composed-prompt formula in F04.** `system_prompt + "\n\n" + join("\n\n", attached_skill_bodies)`. This matches F07's planned composition (skills appended in attachment order, see PRD §6 F07) without the memory block or conversation tail. F07 will extend the same builder.
4. **Detach has no confirmation** (PRD F04 §6: "Detaching a skill is a one-click action with no confirmation"). Undo is via re-attach from the picker.

## 3. Component Overview

### Repository additions

```
/backend
└── /src
    ├── agents/
    │   └── models.rs              # extended: model_context_chars(provider, model)
    └── skill_attachments/
        ├── mod.rs
        ├── model.rs               # AttachedSkill, ComposePreview, AttachUpsert
        ├── service.rs             # replace_attachments, detach_one, compose
        └── routes.rs              # /api/agents/:id/skills/* + /compose

/frontend
└── /src
    ├── pages/
    │   └── Agents/
    │       ├── AgentSkillsPanel.tsx       # inside AgentForm, edit mode only
    │       ├── AttachSkillsDialog.tsx     # multi-select picker
    │       └── ComposedPromptIndicator.tsx
    ├── hooks/
    │   └── useAgentSkills.ts
    └── lib/
        └── agentSkills.ts                 # typed REST client
```

No migration is required — F01's `0001_init.sql` already created the `agent_skills` table with every constraint F04 needs.

## 4. Data Model

### Existing table (F01, unchanged)

```sql
CREATE TABLE agent_skills (
    agent_id   UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    skill_id   UUID NOT NULL REFERENCES skills (id) ON DELETE CASCADE,
    position   SMALLINT NOT NULL CHECK (position BETWEEN 0 AND 19),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, skill_id),
    UNIQUE (agent_id, position)
);
CREATE INDEX idx_agent_skills_skill ON agent_skills (skill_id);
```

The `UNIQUE(agent_id, position)` constraint forces F04 to use a transaction (delete-then-insert) for replace-and-reorder, since intermediate states would otherwise violate the constraint.

### Domain types

```rust
pub struct AttachedSkill {
    pub skill_id: Uuid,
    pub name: String,
    pub description: String,
    pub position: i16,
}

pub struct AttachReplaceInput {
    pub skill_ids: Vec<Uuid>,   // order = future position
}

pub struct ComposePreview {
    pub composed: String,
    pub length_chars: i64,
    pub model_context_chars: i64,
    pub fraction: f32,           // length_chars / model_context_chars
    pub warning: Option<String>, // populated if fraction >= 0.8
}
```

## 5. REST API

All responses use the F01 JSON error envelope; all bodies are JSON.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/agents/:id/skills` | Ordered list of attached skills for the agent |
| PUT | `/api/agents/:id/skills` | Replace + reorder the full attachment list in one TX |
| DELETE | `/api/agents/:id/skills/:skill_id` | Detach a single skill (preserves remaining order) |
| GET | `/api/agents/:id/compose` | Server-composed prompt preview + length budget |

### GET `/api/agents/:id/skills`

```json
{
  "attached": [
    { "skill_id": "uuid", "name": "Concise", "description": "Keep replies tight", "position": 0 },
    { "skill_id": "uuid", "name": "Cite Sources", "description": "...", "position": 1 }
  ]
}
```

### PUT `/api/agents/:id/skills`

Request:

```json
{ "skill_ids": ["uuid-a", "uuid-b", "uuid-c"] }
```

Behavior: inside a single TX, `DELETE FROM agent_skills WHERE agent_id = $1` then re-insert with `position = array_index`. Returns the same payload as `GET`. Duplicate `skill_id`s in the request body → 422 `validation_failed`. More than 20 → 422 `too_many_skills`. Any `skill_id` that does not exist → 422 `unknown_skill`. The deletion path is FK-safe (no other table references `agent_skills`).

### DELETE `/api/agents/:id/skills/:skill_id`

Removes one row, then **renumbers** remaining positions to close the gap (same TX). Returns 204. Detaching a skill the agent does not currently have → 404 `not_attached`.

### GET `/api/agents/:id/compose`

```json
{
  "composed": "<system_prompt>\n\n<skill_body_0>\n\n<skill_body_1>",
  "length_chars": 1284,
  "model_context_chars": 800000,
  "fraction": 0.0016,
  "warning": null
}
```

When `fraction >= 0.8`, `warning` is set to `"composed prompt is at <pct>% of model budget — consider shortening or detaching skills"`.

### Soft-validation envelope

Successful `PUT` returns:

```json
{
  "attached": [ ... ],
  "warnings": [
    { "field": "composed", "message": "composed prompt is at 87% of model budget" }
  ]
}
```

`warnings` is always present (possibly empty) so the frontend can branch uniformly. The `composed` warning is computed inside the PUT handler immediately after the TX commits.

## 6. Validation Rules

| Rule | On violation |
|---|---|
| `skill_ids.len() <= 20` | 422 `too_many_skills` |
| `skill_ids` contains no duplicates | 422 `validation_failed` (`field: skill_ids`) |
| Every `skill_id` exists in `skills` | 422 `unknown_skill` |
| Agent `:id` exists | 404 `agent_not_found` |
| Composed prompt length ≥ 80 % of model budget | **warning** only (never blocks) |

The 0–20 bound matches the DB `position BETWEEN 0 AND 19` constraint (max 20 rows per agent).

## 7. Frontend

### Agent detail page integration

F02's `AgentForm` (edit mode) gains a new section between the system-prompt textarea and the sticky footer:

```
┌── Skills ─────────────────────────────────────┐
│  [ ⋮⋮ ] Concise          ⨯                   │
│  [ ⋮⋮ ] Cite Sources     ⨯                   │
│  [ + Attach skills ]                          │
│                                                │
│  Composed prompt: 1,284 / 800,000 chars (0%)  │
└────────────────────────────────────────────────┘
```

- Drag handle (`⋮⋮`) and a row for each attached skill. Up/down keyboard shortcuts move the focused row (PRD requires keyboard reorder for accessibility).
- The trash icon (`⨯`) calls the per-row DELETE endpoint with optimistic UI; failure rolls back and surfaces a toast.
- The footer of the section shows `ComposedPromptIndicator`: chars used, total budget, percentage, and a yellow warning chip at ≥ 80 %, red at ≥ 95 %.

### Attach modal (`AttachSkillsDialog.tsx`)

- Lists every skill in the workspace (calls `GET /api/skills`); rows display name + description + a checkbox.
- Already-attached skills are pre-checked and visually muted.
- Search input filters by name + description (case-insensitive contains).
- "Attach selected" CTA assembles the final ordered list (existing order preserved, newly checked items appended in alphabetical name order) and sends `PUT /api/agents/:id/skills`.
- Background-click and Esc both dismiss (no destructive action).

### Reorder UX

- Drag-and-drop uses `@dnd-kit/core` (already a dependency in `package.json` from F02 attempts; if absent, add it as part of this phase).
- Each drag end (or up/down key) optimistically reorders the local list and fires `PUT /api/agents/:id/skills`. Errors revert.
- React Query keys: queries `['agent-skills', agentId]` and `['agent-compose', agentId]`. Both are invalidated on every mutation; `['skills']` is also invalidated so the F03 list's `attached_agent_count` badge refreshes.

### Composed-prompt indicator (`ComposedPromptIndicator.tsx`)

A small status row under the Skills section. Polls `GET /api/agents/:id/compose` via TanStack Query (no auto-refetch; invalidated by the attach/detach mutations and by edits to the agent's system prompt or any attached skill body). Renders:

- `1,284 / 800,000 chars (0%)` in neutral text under 80 %.
- A yellow Alert at 80–94 % with the server-supplied warning string.
- A red Alert at ≥ 95 % saying "near model context limit — chat may fail".

## 8. Error Handling

| Scenario | Backend response | Frontend behavior |
|---|---|---|
| Agent not found | 404 `agent_not_found` | Toast `Agent no longer exists`, navigate to `/agents` |
| Unknown skill in PUT body | 422 `unknown_skill` (with `details.skill_id`) | Inline error above the Skills section; UI rolls back |
| Duplicate skill in PUT body | 422 `validation_failed` | Inline error; UI rolls back |
| > 20 skills in PUT body | 422 `too_many_skills` | Toast `An agent can have at most 20 skills`; UI rolls back |
| Detach a skill not attached | 404 `not_attached` | Silent rollback (we already know it isn't there) + toast |
| DB error on attach/detach | 500 `db_error` | Toast `Update failed — retry?`; UI rolls back to last known good list |
| Compose query DB error | 500 | Indicator shows `—`; section still functional |
| Composed prompt ≥ 80 % | 200 with warning | Yellow/red Alert; **chat still allowed** per PRD |

## 9. Testing Strategy

**Backend unit (`skill_attachments::service::tests`):**
- `replace_attachments` rejects duplicates, > 20, and unknown skill IDs
- `replace_attachments` produces sequential `position` 0..N-1
- `detach_one` renumbers remaining positions with no gaps
- `compose` returns the documented concatenation and length math
- `model_context_chars` returns the documented per-provider defaults

**Backend integration (sqlx test pool against the docker-compose Postgres):**
- Create an agent + two skills (via F02/F03 services); attach both → list returns both in order; reorder → list reflects new order; detach one → remaining row's position is renumbered to 0
- Replace with 21 skills → 422 `too_many_skills`
- Replace with a duplicate skill ID → 422 `validation_failed`
- Replace with a skill ID that does not exist → 422 `unknown_skill`
- Cascade: deleting the agent or the skill removes the corresponding `agent_skills` rows (FK)
- `GET /:id/compose` returns `system_prompt + "\n\n" + body0 + "\n\n" + body1` exactly, with `length_chars` equal to the byte count and `fraction` between 0 and 1

**Frontend (Vitest + Testing Library):**
- `AgentSkillsPanel` renders the attached list in order from a mocked GET
- Detach button on a row triggers the DELETE endpoint and removes the row optimistically
- `AttachSkillsDialog` shows all skills, pre-checks already-attached ones, and sends the merged ordered list on confirm
- `ComposedPromptIndicator` renders neutral / yellow / red states for fractions 0.1, 0.85, 0.97
- A failed PUT rolls back the local order

**Manual smoke (documented in `verify.md`):** create two skills via F03 → open an agent in F02 → attach both via the modal → reorder → detach one → reload (verify persistence) → bump a skill body in F03 close to model budget and verify the indicator turns yellow.

## 10. Acceptance Criteria Mapping

PRD §9 F04 → spec sections:

| Criterion | Section |
|---|---|
| Attach multiple skills via picker, listed in attachment order | §5 PUT, §7 AttachSkillsDialog |
| Reorder changes concatenation order | §5 PUT (positions), §7 drag-and-drop / keyboard, §9 backend integration test |
| Detach removes attachment without deleting the skill | §5 DELETE, §7 trash icon |
| Composed-prompt warning when approaching model context | §5 GET /compose, §7 ComposedPromptIndicator |
| 0 to 20 attached skills | §6 too_many_skills + DB `position 0..19` |

Cross-feature criteria from PRD §9 covered here:
- "Skills created in F03 appear in the F04 picker and are concatenated in attachment order" → §7 AttachSkillsDialog + §5 GET /compose
- "Editing a skill body in F03 changes the next composed prompt" → §5 GET /compose always reads `body` fresh from `skills` (no caching in F04), verified by the smoke checklist
