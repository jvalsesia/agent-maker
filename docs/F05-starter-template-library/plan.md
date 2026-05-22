# F05. Starter Template Library — Implementation Plan

## Prerequisites

- F01–F04 are on `main` (`docker compose up -d`, `cargo run`, `pnpm dev`)
- F02 `agents` table + service, F03 `skills` table + service, F04 `agent_skills` writes are all available — F05 reuses them on adopt
- No new migration is required; templates are compile-time data
- `skills-example.md` at the repo root is the authoritative human-readable source for the 10 skill bodies

## Phase 1 — Backend Catalog and Service

1. **Module scaffold (`backend/src/templates/`)** — Create `mod.rs`, `model.rs`, `catalog.rs`, `service.rs`. Wire `pub mod templates;` from `lib.rs` next to `agents`, `skills`, and `skill_attachments`. Mirror the F03/F04 module shape.

2. **Domain types (`model.rs`)** — Define `Category` (serde lowercase enum), `SkillTemplate`, `AgentTemplate`, `AdoptedAgent`, `AdoptedSkill`, and the public JSON envelopes (`TemplatesList`, `AgentTemplateDetail`, `AdoptAgentResponse`, `AdoptSkillResponse`). All `&'static str` everywhere to keep the catalog cheap at runtime.

3. **Static catalog (`catalog.rs`)** — Declare `pub const STARTER_SKILLS: &[SkillTemplate]` with the 10 entries from `skills-example.md` (bodies copied verbatim, one Rust constant per entry referenced by `STARTER_SKILLS`). Declare `pub const STARTER_AGENTS: &[AgentTemplate]` with the 10 entries from spec §4. Add a `pub fn validate() -> Result<(), &'static str>` enforcing the six rules in spec §6.

4. **`templates::service`** — Implement `list(category: Option<Category>) -> TemplatesList`, `get_agent(slug) -> Option<AgentTemplateDetail>`, `get_skill(slug) -> Option<SkillTemplateDetail>`, `adopt_agent(slug) -> AppResult<AdoptedAgent>`, and `adopt_skill(slug) -> AppResult<AdoptedSkill>`. Both `adopt_*` run inside a single `BEGIN/COMMIT` and use the F02/F03 SQL helpers directly (or a small shared `unique_name` routine) to insert rows and bump the " (template)" suffix on `23505` retries. `adopt_agent` walks `suggested_skills` in order: re-use an existing user skill by name when present, else recursively adopt the matching skill template, then write `agent_skills` rows with sequential `position`.

5. **Startup validation** — Call `templates::catalog::validate().expect("template catalog invalid")` from `lib.rs::build_app` and `build_app_with_store` so a bad catalog fails fast rather than blowing up the first user request.

## Phase 2 — Backend REST Surface

6. **`templates::routes`** — Mount five handlers under `/api/templates*` per spec §5. Use Axum extractors for path/query. Map `TemplateNotFound` → 404, DB failures → 500 with the F01 envelope. Reuse F03/F04's `Warnings` JSON shape (always present, possibly empty).

7. **Wire routes into the Axum router** — Add `templates::routes::router()` under the existing `/api` `Router::nest` in `routes/mod.rs`, alongside `agents`, `skills`, and `attachments`. Add a `templates: TemplatesService` field to `AppState`.

8. **Integration tests (`tests/templates.rs`)** — Cover list, filtered list, preview, 404 on bogus slug, end-to-end adopt-agent on an empty workspace, adopt-agent re-using an existing user skill, double-adopt-agent collision suffix, adopt-skill, and adopt rollback. Use the same disposable-Postgres harness established in F02/F03/F04.

## Phase 3 — Frontend

9. **REST client (`lib/templates.ts`)** — Strongly typed wrappers (`listTemplates`, `getAgentTemplate`, `getSkillTemplate`, `adoptAgent`, `adoptSkill`). All functions throw the same typed `ApiError` used by F02/F03/F04 so components can branch on `code`.

10. **React Query hooks (`useTemplates.ts`)** — A single `useTemplates(category?)` query plus per-slug preview queries and two adopt mutations. Successful adopt-agent invalidates `['agents']`, `['skills']`, and `['agent-skills', newId]`. Adopt-skill invalidates `['skills']`.

11. **`TemplatesGallery.tsx`** — Sticky header with search input, single-select category chips (All / Writing / Research / Productivity / Coding / Learning / Wellbeing), and an agent/skill segmented toggle. Body grid renders cards per the spec layout; client-side filter combines search + chip + toggle. Empty state copy + "Clear filters" link when zero matches.

12. **`TemplatePreviewDrawer.tsx`** — Reuses the existing shadcn `dialog` primitive as a right-side sheet. Shows the full prompt body in a monospace block with copy-to-clipboard. For agents, lists the suggested skills inline. Sticky footer with `Adopt` and `Close`.

13. **Routing and navigation** — Add `/templates` to `router.tsx`. Activate the existing left-nav "Templates" item from F01. Empty-state CTA on `/templates` is the same Adopt button; first-adopt success toast offers `Open agent` / `Open skill` shortcut.

## Phase 4 — Verification

14. **Frontend tests** — Implement the Vitest + Testing Library cases enumerated in spec §9 under `frontend/src/pages/Templates/`. Reuse the MSW-style mock-fetch handler conventions established in F02/F03/F04; add `/api/templates/*` handlers.

15. **Manual end-to-end smoke** — With the full stack running and an empty workspace: open `/templates` → confirm 20 cards visible (10 agents + 10 skills) → filter by `Writing` → narrow further by typing in the search box → preview `Writing Editor` (verify the system-prompt body renders in the drawer) → adopt → confirm toast with `Open agent` deep-link → land on `/agents/:id` and verify two skills attached in the documented order → re-adopt the same template → confirm the second copy is named `Writing Editor (template)` → adopt a skill directly (`Concise Replies`) → confirm it appears in `/skills`. Capture the steps in `docs/F05-starter-template-library/verify.md`.
