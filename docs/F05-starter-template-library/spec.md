# F05. Starter Template Library — Technical Specification

## 1. Overview

F05 ships a **read-only catalog** of 10 starter agents and 10 starter skills bundled with the app at compile time, so a brand-new user sees real examples instead of an empty workspace. The catalog is browsable offline (no network call) and exposed via `/api/templates/*`. Adopting a template creates an **editable copy** of the agent or skill in the user's workspace via the same F02/F03 services that power CRUD — F05 owns the catalog, not a parallel persistence layer. When an agent template is adopted, its suggested skill attachments are also resolved (re-using existing user skills with matching names, adopting missing ones) and wired up via F04's `agent_skills` insert. Name collisions on adopt are resolved by suffixing `" (template)"` per PRD §6 F05.

## 2. Scope

**Included:**
- A `templates::catalog` module with `const`-declared starter agents and starter skills compiled into the binary (no I/O, no migration). Each entry has a stable kebab-case `slug` used as the URL key.
- REST endpoints under `/api/templates/*` for list (with optional `?category=` filter), preview, and adopt
- Adopt-agent flow that (a) inserts an editable agent copy in the F02 `agents` table, (b) ensures each suggested skill exists in the user's workspace (re-use by name, or adopt the matching skill template if missing), (c) writes ordered `agent_skills` rows via the F04 attachment service
- Adopt-skill flow that inserts a single editable skill copy via the F03 skills service
- Name-collision handling: " (template)" suffix appended, with " (template) (n)" retry on further collisions (mirrors F02/F03 clone-naming logic)
- Frontend: a new `/templates` route with category filter chips, a search box, agent/skill toggle, and per-card Preview / Adopt actions; preview drawer renders the full prompt body
- Activate the existing left-nav "Templates" placeholder from F01

**Integrated from PRD blocks:**
- `Capabilities`: ≥ 10 starter agents and ≥ 10 starter skills across the Writing / Research / Productivity / Coding / Learning / Wellbeing categories; adoption produces an editable copy; library browsable offline; templates versioned by the app (user copies are independent)
- `Experience`: gallery with category filter chips, search box, preview drawer with full prompt body, "Adopt" CTA, toast offering "Open agent" / "Open skill" shortcut after adoption
- `Error Handling`: name collisions suffix " (template)"; DB write failure leaves nothing partially created (single TX)

**Deferred (out of F05):**
- Template versioning beyond the bundled snapshot (PRD mentions versioning but no migration path between bundled-template versions — out of scope for v1; user copies are independent so this is safe)
- Sharing or publishing user-created templates back to the library (PRD §7 explicitly out of scope)
- A template-update prompt when bundled templates change in a future release

**Assumptions / decisions (PRD did not specify):**
1. **Templates as compile-time `const` data.** The 10 agent + 10 skill templates live in `backend/src/templates/catalog.rs` as `pub const STARTER_AGENTS: &[AgentTemplate]` and `pub const STARTER_SKILLS: &[SkillTemplate]`. No file I/O at runtime. `skills-example.md` at the repo root is the authoritative human-readable source for the 10 skills; the Rust data mirrors it byte-for-byte, with a comment on each entry pointing to the markdown header.
2. **Slug = kebab-cased template name.** `"Concise Replies"` → `concise-replies`. Slugs are stable (a content edit may change a name's display but slugs should be edited only via a new release). Slug appears in URLs (`/api/templates/agents/:slug`) and is unique across agent and skill spaces.
3. **Suggested-skill resolution on adopt-agent.** For each suggested skill slug on the agent template: if a user skill with the matching template name (or its slug-matched name) already exists, attach it. Otherwise look up the matching skill template by slug and adopt it (creating a new editable copy with " (template)" suffixing as needed), then attach the resulting skill. The composed attachment order matches the template's suggested order.
4. **Adopt is single-transaction.** The whole adopt-agent flow (agent insert + 0-N skill inserts + agent_skills inserts) runs inside one `BEGIN/COMMIT`. Any failure rolls everything back so the workspace is never partially populated.
5. **No `last_used_at` bump.** Adoption is a creation event, not a use event. The new agent's `last_used_at` stays NULL until F07 dispatches a real chat.

## 3. Component Overview

### Repository additions

```
/backend
└── /src
    └── templates/
        ├── mod.rs
        ├── model.rs                # AgentTemplate, SkillTemplate, AdoptedAgent, AdoptedSkill
        ├── catalog.rs              # const STARTER_AGENTS / STARTER_SKILLS
        ├── service.rs              # list, preview, adopt_agent, adopt_skill
        └── tests.rs                # validation: every template parses, slugs unique, suggested skills resolve

/frontend
└── /src
    ├── pages/
    │   └── Templates/
    │       ├── TemplatesGallery.tsx
    │       ├── TemplatePreviewDrawer.tsx
    │       └── index.ts
    ├── hooks/
    │   └── useTemplates.ts
    └── lib/
        └── templates.ts            # typed REST client wrappers
```

The left-nav "Templates" item (placeholder from F01) becomes active and routes to `/templates`.

## 4. Data Model

F05 adds **no migrations**. Templates are compile-time data. Adoption writes to existing F01/F02/F03/F04 tables (`agents`, `skills`, `agent_skills`).

### Domain types

```rust
pub struct SkillTemplate {
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: Category,
    pub body: &'static str,
}

pub struct AgentTemplate {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub preamble: &'static str,
    pub system_prompt: &'static str,
    pub default_provider: &'static str,    // e.g., "anthropic"
    pub default_model: &'static str,       // e.g., "claude-haiku-4-5"
    pub suggested_skills: &'static [&'static str], // skill slugs in attachment order
}

pub enum Category {
    Writing,
    Research,
    Productivity,
    Coding,
    Learning,
    Wellbeing,
}

pub struct AdoptedAgent {
    pub agent: crate::agents::model::Agent,
    pub attached_skill_ids: Vec<Uuid>,
}

pub struct AdoptedSkill {
    pub skill: crate::skills::model::Skill,
}
```

### Starter catalog (10 + 10)

Skills (from `skills-example.md`, in display order):

| Slug | Name | Category |
|---|---|---|
| `concise-replies` | Concise Replies | Writing |
| `cite-sources` | Cite Sources | Research |
| `socratic-tutor` | Socratic Tutor | Learning |
| `code-reviewer` | Code Reviewer | Coding |
| `meeting-prep-coach` | Meeting Prep Coach | Productivity |
| `research-assistant` | Research Assistant | Research |
| `blunt-editor` | Blunt Editor | Writing |
| `sql-helper` | SQL Helper | Coding |
| `decision-framer` | Decision Framer | Productivity |
| `wellbeing-check-in` | Wellbeing Check-In | Wellbeing |

Agents (each paired with 1–3 suggested skill slugs):

| Slug | Name | Category | Suggested skills |
|---|---|---|---|
| `writing-editor` | Writing Editor | Writing | `blunt-editor`, `concise-replies` |
| `email-drafter` | Email Drafter | Writing | `concise-replies` |
| `research-analyst` | Research Analyst | Research | `research-assistant`, `cite-sources` |
| `fact-checker` | Fact Checker | Research | `cite-sources` |
| `meeting-prep` | Meeting Prep | Productivity | `meeting-prep-coach`, `concise-replies` |
| `decision-coach` | Decision Coach | Productivity | `decision-framer` |
| `code-mentor` | Code Mentor | Coding | `code-reviewer`, `cite-sources` |
| `sql-buddy` | SQL Buddy | Coding | `sql-helper` |
| `study-tutor` | Study Tutor | Learning | `socratic-tutor`, `cite-sources` |
| `journal-companion` | Journal Companion | Wellbeing | `wellbeing-check-in` |

All agent templates default to `provider = "anthropic"`, `model = "claude-haiku-4-5"` (the cheapest/fastest Claude in F02's `available_models`). Users can change either after adoption.

## 5. REST API

All responses use the F01 JSON error envelope. All bodies are JSON.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/templates` | List all templates (agents + skills), optional `?category=` filter |
| GET | `/api/templates/agents/:slug` | Full preview of one agent template |
| GET | `/api/templates/skills/:slug` | Full preview of one skill template |
| POST | `/api/templates/agents/:slug/adopt` | Adopt agent template (and suggested skills) |
| POST | `/api/templates/skills/:slug/adopt` | Adopt single skill template |

### GET `/api/templates`

```json
{
  "agents": [
    {
      "slug": "writing-editor",
      "name": "Writing Editor",
      "category": "writing",
      "preamble": "A sharp, opinionated writing editor.",
      "suggested_skills": ["blunt-editor", "concise-replies"]
    }
  ],
  "skills": [
    {
      "slug": "concise-replies",
      "name": "Concise Replies",
      "category": "writing",
      "description": "Keep responses tight, skimmable, and free of filler."
    }
  ]
}
```

Optional query: `?category=writing` filters both lists to that category.

### GET `/api/templates/agents/:slug`

```json
{
  "agent": {
    "slug": "writing-editor",
    "name": "Writing Editor",
    "category": "writing",
    "preamble": "A sharp, opinionated writing editor.",
    "system_prompt": "You are a sharp …",
    "default_provider": "anthropic",
    "default_model": "claude-haiku-4-5",
    "suggested_skills": [
      { "slug": "blunt-editor", "name": "Blunt Editor", "description": "…" },
      { "slug": "concise-replies", "name": "Concise Replies", "description": "…" }
    ]
  }
}
```

### POST `/api/templates/agents/:slug/adopt`

Empty body. Returns:

```json
{
  "agent": { ...Agent... },
  "attached_skill_ids": ["uuid-1", "uuid-2"],
  "warnings": []
}
```

Behavior, all inside one TX:
1. Resolve final agent name. Start with the template name; if a user agent with that name exists, append `" (template)"`. If `"<name> (template)"` also exists, try `" (template) (2)"`, then `(3)`, … up to 999.
2. Insert the agent row via the F02 service helper (reuses validation).
3. For each suggested-skill slug, in order:
   - If a user skill exists whose name matches the template's name (or its " (template)" variant), reuse its id.
   - Else look up the skill template by slug, derive a final name via the same suffix algorithm, and insert it via F03's service helper.
4. Insert `agent_skills` rows with `position = index`.

If any step fails the whole TX rolls back; HTTP 500 with `db_error`.

### POST `/api/templates/skills/:slug/adopt`

Empty body. Returns:

```json
{ "skill": { ...Skill... }, "warnings": [] }
```

Adopts a single skill with " (template)" suffixing on collision.

### Errors

| Scenario | Status | code |
|---|---|---|
| Unknown agent slug | 404 | `template_not_found` |
| Unknown skill slug | 404 | `template_not_found` |
| Suggested skill slug missing from catalog (internal data bug) | 500 | `catalog_error` |
| DB write failure | 500 | `database_error` |

## 6. Validation Rules

All template data is validated **at startup** by a `templates::catalog::validate()` call from `lib.rs::build_app`:

| Rule | On violation |
|---|---|
| Every agent + skill slug is unique within its kind | panic on startup (programmer error) |
| Each agent's `suggested_skills` references existing skill slugs | panic on startup |
| Each template name is 1–60 chars (matches F02/F03 hard limit) | panic on startup |
| Each system_prompt is 1–20,000 chars (matches F02 hard limit) | panic on startup |
| Each skill body is 1–10,000 chars (matches F03 hard limit) | panic on startup |
| Each agent has ≤ 20 suggested skills (matches F04 attachment cap) | panic on startup |

Panic-on-startup is the right move because this is bundled compile-time data — any violation is a code bug, not a user error.

There is no runtime input validation; adopt endpoints accept empty bodies.

## 7. Frontend

### Routes

- `/templates` → `TemplatesGallery` with sticky header (search + category chips + agent/skill toggle)

### Layout

```
┌─ Templates ─────────────────────────────────────────────┐
│  [ search… ]                                            │
│  [ All ] [ Writing ] [ Research ] [ Productivity ] [Coding] [Learning] [Wellbeing]
│  Showing: [Agents]  [Skills]                            │
├──────────────────────────────────────────────────────────┤
│  ┌── Writing Editor ──────────┐  ┌── Email Drafter ───┐ │
│  │  A sharp, opinionated …    │  │  …                 │ │
│  │  Writing                   │  │  Writing           │ │
│  │  [ Preview ] [ Adopt ]     │  │  [ Preview ][Adopt]│ │
│  └────────────────────────────┘  └────────────────────┘ │
│  …                                                      │
└──────────────────────────────────────────────────────────┘
```

### Preview drawer (`TemplatePreviewDrawer.tsx`)

A right-side sheet (reusing the F02/F03 `dialog` primitive if no `sheet` is wired) showing:
- Name, category badge, preamble (agents only) or description (skills only)
- Full prompt body in a monospace block (read-only `<pre>` with copy-to-clipboard)
- For agents: the list of suggested skills (name + description, no actions)
- Sticky footer with `Adopt` and `Close`

### Adopt flow

- `Adopt` button calls the adopt mutation.
- Success → shadcn toast `Adopted "<final name>"` with an inline button **`Open agent`** / **`Open skill`** that deep-links to `/agents/:id` or `/skills/:id`.
- Mutation invalidates `['agents']`, `['skills']`, and (for agent adopts) `['agent-skills', new_id]`.

### Category filter

Chips toggle a single active category at a time (default: All). The agent/skill toggle is a segmented control showing both by default; user can flip to agents-only or skills-only when the list gets long.

### Search

Case-insensitive contains on name + description (skills) / preamble (agents). Filtering is client-side over the single `GET /api/templates` payload (the catalog is bounded at 20 entries, no need for server-side search).

### Empty-state copy

If a filter combination produces zero matches: "No templates match these filters." with a "Clear filters" link.

## 8. Error Handling

| Scenario | Backend response | Frontend behavior |
|---|---|---|
| Unknown slug | 404 `template_not_found` | Toast `Template no longer exists`; gallery list refreshes |
| DB error on adopt | 500 `database_error` | Toast `Couldn't adopt — please try again`; nothing partially created |
| Name collision (handled internally) | 200, response carries final suffixed name | Toast shows the final name so the user sees what was created |
| Catalog data bug | App fails to start (panic during `build_app`) | Developer sees the panic in the logs; user never sees it |

The PRD's `Error Handling` for F05 — "adopt fails (DB write error): toast with retry; nothing partially created" and "name collision on adopt: adopted copy is suffixed (template)" — maps directly to the two non-trivial rows above.

## 9. Testing Strategy

**Backend unit (`templates::tests`):**
- `validate_catalog` returns Ok for the bundled catalog (covers all 6 hard rules above)
- Every agent's `suggested_skills` slug resolves to an entry in `STARTER_SKILLS`
- `final_name` helper appends " (template)", " (template) (2)", " (template) (3)" on repeated collisions

**Backend integration (sqlx test pool against the docker-compose Postgres):**
- `GET /api/templates` returns 10 agents and 10 skills with the documented shape
- `GET /api/templates?category=writing` returns only the Writing entries from both lists
- `GET /api/templates/agents/writing-editor` returns the full preview with `suggested_skills`
- `GET /api/templates/agents/<bogus>` returns 404 `template_not_found`
- `POST /api/templates/agents/writing-editor/adopt` against an empty workspace creates one new agent + two new skills + two `agent_skills` rows in attachment order
- `POST /api/templates/agents/writing-editor/adopt` against a workspace where `Blunt Editor` already exists (as a user-created skill) reuses that existing skill (no duplicate) and attaches it
- `POST` adopt twice for the same template produces `Writing Editor` then `Writing Editor (template)` (collision suffix)
- `POST /api/templates/skills/concise-replies/adopt` creates a single skill
- Adopt failure (simulate by setting a unique constraint on the partial write) rolls back atomically — the workspace remains as it was

**Frontend (Vitest + Testing Library):**
- `TemplatesGallery` renders the 10 + 10 entries from a mocked GET; category chip filters narrow the visible cards
- Search input narrows by name + description
- Clicking `Preview` opens the drawer and renders the prompt body
- Clicking `Adopt` on a card fires the mutation and shows the success toast with the deep-link button

**Manual smoke (documented in `verify.md`):** open `/templates` against a fresh workspace → confirm 20 cards visible → filter by Writing → adopt `Writing Editor` → confirm toast → click `Open agent` → confirm two skills are attached in the documented order → re-adopt and confirm the second copy is suffixed.

## 10. Acceptance Criteria Mapping

PRD §9 F05 → spec sections:

| Criterion | Section |
|---|---|
| ≥ 10 starter agents and ≥ 10 starter skills out of the box | §4 catalog tables |
| Category filter narrows the list | §7 chips |
| Adopting creates editable copy without modifying the template | §5 POST adopt (catalog is read-only `const`) |
| Adoption succeeds offline | §1 (compile-time data, no network) |
| Name collision suffixes " (template)" | §5 final_name algorithm, §6 validation |

Cross-feature criteria from PRD §9 covered here:
- "Adopting a template in F05 creates an editable agent or skill that is then manageable via F02 or F03" — §5 adopt endpoints reuse the F02/F03 services and produce native `Agent` / `Skill` rows
