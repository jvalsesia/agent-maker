# F03. Skill Management — Technical Specification

## 1. Overview

F03 adds full CRUD plus cloning for skills — the reusable instruction bundles attached to agents. A skill is a row in the `skills` table (already created by F01) carrying a name, description, and instruction body. F03 implements the REST surface and the list/detail/edit UI. It does **not** implement attachment to agents (F04) — but it does expose the per-skill `attached_agent_count` and the list of using agents so the F03 UI can render the "Used by N agents" panel and the delete-confirmation modal mandated by PRD §6 F03.

## 2. Scope

**Included:**
- REST endpoints under `/api/skills/*` for list, get, create, update, delete, clone
- Sort and search on the list endpoint (`?sort=name|attached|created&order=asc|desc&q=<text>`)
- Soft validation envelope mirroring F02: warnings (instruction body shorter than 50 chars or longer than 5,000 chars) returned alongside the saved entity, never blocking the save
- Hard validation: name length 1–60 and unique, description 1–200 required, body 1–10,000 required
- A `using_agents` list on `GET /api/skills/:id` returning the agents currently attached to the skill via `agent_skills`
- Frontend pages: `/skills` (list with search + sort + "+ New"), `/skills/new`, `/skills/:id` (detail/edit), and the delete/clone modals
- Delete confirmation modal lists every agent currently using the skill, cannot be dismissed by background click (PRD requirement)
- Activate the existing left-nav "Skills" placeholder from F01

**Integrated from PRD blocks:**
- `Capabilities`: field bounds, CRUD + clone, immediate propagation (no caching of bodies in F03), sortable list by name and attached count
- `Experience`: skills list in left nav parallel to Agents, single-page form, "Used by N agents" panel, delete modal listing using agents
- `Error Handling`: duplicate-name inline error, no-dismiss attached-skill delete modal, save failure toasts

**Deferred (out of F03):**
- The actual `agent_skills` insert/update/reorder UI and endpoints (F04)
- F03 only **reads** `agent_skills` to render counts and the using-agents list — it never writes to it
- Cross-feature criterion from PRD §9 "skill body change reflects in next compose" is observable but its end-to-end verification belongs to F07

## 3. Component Overview

### Repository additions

```
/backend
├── /migrations
│   └── 0004_skills_touch.sql       # adds BEFORE UPDATE trigger on skills; no schema change
└── /src
    └── skills/
        ├── mod.rs
        ├── model.rs                # Skill, SkillSummary, SkillUpsert, validation errors
        ├── service.rs              # business logic, clone, using-agents lookup
        └── routes.rs               # /api/skills/* handlers

/frontend
└── /src
    ├── pages/
    │   └── Skills/
    │       ├── SkillsList.tsx
    │       ├── SkillForm.tsx       # used by /new and /:id
    │       ├── DeleteSkillDialog.tsx
    │       └── index.ts
    ├── hooks/
    │   ├── useSkills.ts
    │   └── useSkill.ts
    └── lib/
        └── skills.ts               # typed REST client
```

The left-nav "Skills" item (placeholder from F01) becomes active and routes to `/skills`.

## 4. Data Model

F01's `skills` table already covers F03 in full. F03 adds **no** column changes — only a trigger to bump `updated_at` on UPDATE (migration `0004_skills_touch.sql`):

```sql
CREATE TRIGGER skills_touch BEFORE UPDATE ON skills
  FOR EACH ROW EXECUTE FUNCTION touch_updated_at();
```

The `touch_updated_at()` function was already created by F02's migration `0003_agents_touch.sql` — F03 only registers the new trigger.

### Domain types

```rust
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SkillSummary {            // list payload
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attached_agent_count: i64,
}

pub struct SkillDetail {             // GET /:id payload
    pub skill: Skill,
    pub using_agents: Vec<UsingAgent>,
}

pub struct UsingAgent {
    pub id: Uuid,
    pub name: String,
}

pub struct SkillUpsert {             // create + update input
    pub name: String,
    pub description: String,
    pub body: String,
}
```

## 5. REST API

All responses use the F01 JSON error envelope. All bodies are JSON.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/skills` | List skills with search/sort |
| POST | `/api/skills` | Create skill |
| GET | `/api/skills/:id` | Get skill plus `using_agents` |
| PUT | `/api/skills/:id` | Update fields (full upsert) |
| DELETE | `/api/skills/:id` | Delete skill (cascades to `agent_skills` via F01 FK) |
| POST | `/api/skills/:id/clone` | Clone skill; body `{ "name": "optional override" }` |

Successful create/update returns:

```json
{
  "skill": { ...Skill... },
  "warnings": [
    { "field": "body", "message": "shorter than 50 chars" }
  ]
}
```

`warnings` is always present (possibly empty).

Hard validation failures return HTTP 422 with `{ "error": { "code": "validation_failed", "details": [...] } }`. Duplicate name returns 409 with `code: "duplicate_name"`.

`POST /api/skills/:id/clone` copies `description` and `body`; new name defaults to `<original> (copy)`, with ` (n)` suffix retry on conflict.

`GET /api/skills` payload:

```json
{
  "skills": [ { ...SkillSummary... } ]
}
```

Supported query params: `q` (case-insensitive contains match on name or description), `sort` in `{name, attached, created}`, `order` in `{asc, desc}` (default: `name asc`).

`GET /api/skills/:id` payload:

```json
{
  "skill": { ...Skill... },
  "using_agents": [ { "id": "uuid", "name": "Writing Editor" } ]
}
```

## 6. Validation Rules

| Field | Rule | On violation |
|---|---|---|
| name | required, 1–60 chars after trim, unique | 422 `validation_failed` or 409 `duplicate_name` |
| description | required, 1–200 chars | 422 |
| body | required, 1–10,000 chars | 422 |
| body | < 50 chars or > 5,000 chars | **warning** only |

Validation runs in `skills::service::validate` before the DB call so the failure shape is consistent across create and update.

## 7. Frontend

### Routes

- `/skills` → `SkillsList` (search input, sort dropdown, "+ New Skill" button)
- `/skills/new` → `SkillForm` in create mode
- `/skills/:id` → `SkillForm` in edit mode (loads via `useSkill`); right-side panel renders `using_agents`

### Form layout (`SkillForm.tsx`)

Single page, top-to-bottom:
1. Name (Input, 60-char counter)
2. Description (Input, 200-char counter)
3. Instruction body (Textarea, full-width, monospace, char counter; soft-validation warnings render inline below the field as muted text)
4. Edit mode only: right-side panel "Used by N agents" listing each using agent with a link to `/agents/:id`
5. Sticky footer: "Cancel", "Save", "Clone" (edit mode only), "Delete" (edit mode only, right-aligned, destructive)

Save uses TanStack Query mutations. Success → shadcn toast `Skill saved` and a brief checkmark on the button.

### List (`SkillsList.tsx`)

Search box + Sort dropdown (`Name`, `Attached agents`, `Created`). Each row: name, truncated description, attached-agent count badge, kebab menu (`Open`, `Clone`, `Delete`). Empty state: "No skills yet — create your first one" with a primary CTA.

### Delete dialog (`DeleteSkillDialog.tsx`)

Modal lists every agent using the skill by name. If the list is empty, the body says "No agents currently use this skill". Buttons: `Cancel` (default focus), `Delete` (destructive). **Background-click is disabled** (PRD requirement); only the Cancel button or Esc dismisses.

## 8. Error Handling

| Scenario | Backend response | Frontend behavior |
|---|---|---|
| Duplicate name | 409 `duplicate_name` | Inline field error under "Name"; form stays dirty |
| Validation failure | 422 with `details[]` | Inline field errors per `field`; form stays dirty |
| DB error on save | 500 `db_error` | Toast `Save failed — retry?`; form stays dirty |
| DB error on delete | 500 | Toast with retry; skill remains in the list (UI rollback) |
| Clone produces non-unique name | server retries suffixing | Cloned skill returned already has the final unique name |

## 9. Testing Strategy

**Backend unit (`skills::service::tests`):**
- `validate` rejects every hard rule and emits warnings for soft rules
- `clone` produces correct ` (copy)` / ` (n)` suffix and copies `description` + `body`

**Backend integration (sqlx test pool against the docker-compose Postgres):**
- Full lifecycle: create → list → get → update → clone → delete; assert row counts and `updated_at` movement (trigger)
- Duplicate-name returns 409 on both create and update
- `GET /api/skills/:id` returns the using-agents list after a fixture inserts an `agent_skills` row
- `GET /api/skills` sort by `attached` and `name` returns rows in the documented order
- `DELETE /api/skills/:id` cascades the `agent_skills` row inserted by the fixture

**Frontend (Vitest + Testing Library):**
- `SkillForm` renders all fields, surfaces validation errors from a mocked 422 response, and disables Save while the mutation is in-flight
- Delete dialog renders the using-agents list and only fires the mutation when the destructive button is clicked
- Background-click does NOT dismiss the delete dialog
- `SkillsList` sort dropdown updates the query and the rendered order

**Manual smoke (documented in `verify.md`):** create → edit → clone → delete a single skill end-to-end against the running stack, including a fixture attachment to verify the using-agents panel.

## 10. Acceptance Criteria Mapping

PRD §9 F03 → spec sections:

| Criterion | Section |
|---|---|
| Create skill with required fields | §5 POST `/api/skills`, §7 Form |
| Editing body propagates to next compose | §1 (no caching), verified at F07 level |
| Clone appends ` (copy)` | §5 clone, §6 retries |
| Delete attached skill requires explicit confirmation with using-agents list | §7 Delete dialog (no-dismiss) |
| Skills list shows attached-agents count, updates on attach/detach | §5 list payload, §7 list rendering |

Cross-feature criteria from PRD §9 #6 (skill body edit changes next composed prompt) are satisfied by always reading `body` fresh in F07; F03 ships no body cache.
