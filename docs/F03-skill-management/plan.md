# F03. Skill Management — Implementation Plan

## Prerequisites

- F01 and F02 are on this branch (`docker compose up -d`, `cargo run`, `pnpm dev`)
- F01's `skills` and `agent_skills` table schemas, JSON error envelope, and shadcn UI primitives are already in place
- F02's `touch_updated_at()` plpgsql function (migration `0003_agents_touch.sql`) exists and is reused

## Phase 1 — Backend Data and Service Layer

1. **Migration `0004_skills_touch.sql`** — Register the `skills_touch` BEFORE UPDATE trigger reusing the existing `touch_updated_at()` function. No column changes. Confirm the migration applies cleanly on top of F02's migrations.

2. **`skills::model` module** — Define `Skill`, `SkillSummary`, `SkillDetail`, `UsingAgent`, `SkillUpsert`, and the `ValidationError` / `Warning` types. Derive `serde::Serialize/Deserialize` and `sqlx::FromRow`. Mirror the F02 `agents::model` shape so the JSON envelopes stay consistent.

3. **`skills::service::validate`** — Centralize the hard + soft validation rules from spec §6. Return `Result<Vec<Warning>, Vec<ValidationError>>` so create and update share the same code path.

4. **`skills::service` CRUD** — Implement `list(filter)`, `get(id)` (joining `agent_skills → agents` to assemble `using_agents`), `create(upsert)`, `update(id, upsert)`, `delete(id)`, and `clone(id, override_name)`. `list` LEFT JOINs `agent_skills` with `GROUP BY skills.id` to produce `attached_agent_count`. `clone` re-runs uniqueness suffixing on `23505` collisions.

## Phase 2 — Backend REST Surface

5. **`skills::routes` handlers** — Mount the six endpoints from spec §5 under `/api/skills`. Use Axum extractors for path/query/body. Map `ValidationError` → 422, `23505` → 409 `duplicate_name`, DB failures → 500 with the F01 envelope. Reuse F02's `Warnings` JSON shape for consistency.

6. **Wire routes into the Axum router** — Add `skills::routes::router()` under the existing `/api` `Router::nest` call in `main.rs`, alongside the F02 `agents` router.

7. **Integration tests (`tests/skills.rs`)** — Cover the lifecycle, duplicate-name handling, sort/search, `using_agents` rendering, and FK cascade per spec §9. Use the same disposable-Postgres harness established in F02.

## Phase 3 — Frontend

8. **REST client (`lib/skills.ts`)** — Strongly typed wrappers (`listSkills`, `getSkill`, `createSkill`, `updateSkill`, `deleteSkill`, `cloneSkill`). All functions throw the same typed `ApiError` used by F02 so components can branch on `code`.

9. **React Query hooks (`useSkills`, `useSkill`)** — Standard query/mutation pairs with invalidation: a successful create/update/delete/clone invalidates `['skills']` and (when applicable) `['skill', id]`. The detail query also invalidates `['agents']` after delete since attachment counts shift (defensive — F04 not yet wired).

10. **Pages and components** — Build `SkillsList`, `SkillForm` (shared by `/skills/new` and `/skills/:id`), and `DeleteSkillDialog` per spec §7. Reuse the same shadcn primitives wired by F02 (`button`, `input`, `label`, `textarea`, `select`, `dialog`, `toast`). The delete dialog disables background-click dismissal.

11. **Routing and navigation** — Add `/skills`, `/skills/new`, `/skills/:id` routes to `router.tsx`. Activate the existing Skills item in the left navigation. Empty-state CTA on `/skills` deep-links to `/skills/new`.

## Phase 4 — Verification

12. **Frontend tests** — Implement the Vitest + Testing Library cases enumerated in spec §9. Reuse the MSW handler conventions established in F02; add `/api/skills/*` handlers.

13. **Manual end-to-end smoke** — With the full stack running: create a skill → reload and confirm persistence → edit it → clone it → attempt a duplicate name (expect inline error) → manually insert one `agent_skills` row via psql → reload detail page (verify using-agents panel + list count badge) → delete (verify modal lists the agent, no background-click dismiss, final disappearance). Capture the steps in `docs/F03-skill-management/verify.md`.
