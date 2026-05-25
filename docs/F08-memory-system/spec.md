# F08. Memory System — Technical Specification

## 1. Overview

F08 turns the existing `messages` and `message_embeddings` tables into a
working memory pipeline: recent-N turns are returned verbatim, older turns
are embedded via OpenAI `text-embedding-3-small` (1536-dim, matching the
existing pgvector column), and the top-K semantically-similar older turns
are surfaced for any new user query. The feature ships a small REST surface
for embedding pending messages, querying the composed memory block, and
clearing long-term memory per conversation — F07 will call these endpoints
when it implements the chat composer. Per-agent overrides for N and K plug
into the agent edit form; the global defaults from F01 settings remain the
fallback.

## 2. Scope

**Included:**
- New backend module `memory/` with an `EmbeddingProvider` trait, an OpenAI
  implementation, and a `MemoryService` that:
  - Embeds every message lacking an embedding for a conversation (idempotent)
  - Resolves the effective `(recent_n, top_k)` per agent (override → global)
  - Returns the recent-N tail + top-K cosine-similar older turns for a
    user query
  - Clears all embeddings for a conversation
- REST endpoints under `/api/conversations/:id/memory*` (see §5)
- Frontend: per-agent N/K controls inside the existing AgentForm "Advanced"
  section, and a "Clear long-term memory" action in the chat surface
  conversation menu
- Embedding-call failure handling: a single failed embedding leaves the
  rest succeeded (per-message try/catch) and surfaces failure counts in the
  endpoint response so callers can decide what to do
- Vector-retrieval failure handling: query falls back to recent-window-only
  with an explicit `degraded: true` flag in the response

**Integrated from PRD blocks:**
- `Consumes`: F06 message history (existing `messages` rows); F01
  persistence layer (`message_embeddings` table already provisioned)
- `Provides`: recent N verbatim turns + top-K semantically retrieved older
  turns per query (consumed by F07 in the next wave)
- `Capabilities`: recent-window with default N=10; embedding-backed older
  retrieval with default K=5; per-conversation clear; async generation;
  bounded N ∈ [4,30], K ∈ [0,10]
- `Experience`: "Recalled N earlier turns" indicator under each assistant
  message (deferred to F07 — F08 ships the data shape); per-agent
  advanced settings expose N/K; conversation menu has "Clear long-term
  memory" with a confirmation showing how many embedded turns will be
  removed
- `Error Handling`: embedding-call failure queues for retry (we use a
  simple idempotent re-embed-pending sweep instead of a queue table);
  vector retrieval failure falls back to recent-window-only with a notice;
  clear failure surfaces a toast

**Deferred (out of F08):**
- "Recalled earlier turns" UI under assistant messages — F07 (F08 returns
  the data; F07 renders it)
- Embedding model selection per provider — single global model
  `text-embedding-3-small` in v1; the model column on `message_embeddings`
  is already present so future migrations can store mixed models
- Local/Anthropic embedding providers — Anthropic exposes no public
  embeddings API; local Ollama support is out of scope until F07 needs it
- Background worker — F07 will trigger embedding by calling
  `POST /api/conversations/:id/embed-pending` after each assistant reply
  finalizes; no Tokio worker in F08

**Assumptions / decisions (PRD did not specify):**
1. **OpenAI is the only embedding provider in v1.** Even if the chat agent
   uses Anthropic, embeddings go through `OPENAI` (cheap, 1536-dim). If no
   OpenAI key is configured, the memory query gracefully degrades to
   recent-window-only and reports `degraded: true`. Documented as a
   single-line decision so a Settings entry can be added in a later release.
2. **Embedding model: `text-embedding-3-small`.** Matches the `vector(1536)`
   column. Stored in `message_embeddings.model` per row so future migrations
   can mix dimensions if needed.
3. **K=0 disables semantic retrieval.** The pgvector query is skipped
   entirely; only the recent-N tail is returned. Matches PRD's
   K ∈ [0,10] bound.
4. **Retrieval excludes the recent-N tail.** Otherwise the recent turns
   would be returned twice (once verbatim, once as a similarity hit). The
   pgvector query uses an `ORDER BY ... LIMIT` over messages that fall
   *outside* the most-recent-N window.
5. **`system` messages are excluded from both windows.** The recent-N tail
   counts user/assistant rows only; the embedded set excludes `system` so
   internal scaffolding never resurfaces as "recalled context".
6. **Embedding is best-effort and idempotent.** The `embed_pending`
   endpoint embeds every user/assistant message in the conversation that
   does not yet have a `message_embeddings` row. Failures are counted and
   returned but do not roll back successful embeddings.
7. **No new migration.** `message_embeddings` is already in
   `0002_pgvector.sql`.

## 3. Component Overview

### Repository additions

```
/backend
└── /src
    ├── memory/
    │   ├── mod.rs
    │   ├── model.rs              # MemoryBlock, RetrievedTurn, EmbedReport,
    │   │                         # MemoryQuery, ClearReport
    │   ├── service.rs            # MemoryService (resolve N/K, embed_pending,
    │   │                         # query, clear)
    │   └── embedding.rs          # EmbeddingProvider trait + OpenAI impl
    └── routes/
        └── memory.rs             # /api/conversations/:id/memory*

/backend/tests
└── memory.rs                     # integration tests (sqlx::test + mockito)

/frontend
└── /src
    ├── lib/api.ts                # extended with memory endpoints
    ├── hooks/useMemory.ts        # query + mutations
    └── pages/
        ├── Agents/AgentForm.tsx  # add N / K controls to the Advanced
        │                         # section (existing collapsible)
        └── Chat/
            └── ConversationMenu.tsx  # already exists or extend sidebar
                                      # to include "Clear long-term memory"
```

### Backend wiring

- `lib.rs::build_app` and `build_app_with_store` instantiate
  `MemoryService::new(pool.clone(), embedding_provider)` and add it to
  `AppState`. The embedding provider is built from the `secrets` store the
  same way `ProviderRegistry` is.
- `routes/mod.rs` merges `memory::routes()` into the `/api` nest.

### Frontend wiring

- `useMemory.ts` exports `useClearMemory(conversationId)` (mutation) and
  `useEmbedPending(conversationId)` (mutation; called by F07 post-reply).
- The F08 PR does **not** add a memory query hook for the chat surface —
  F07 will own that consumer.

## 4. Data Model

F08 adds no migrations. Relevant existing tables:

```sql
-- F01
CREATE TABLE messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'complete',
    model           TEXT,
    token_count     INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- F01 (0002_pgvector.sql)
CREATE TABLE message_embeddings (
    message_id      UUID PRIMARY KEY REFERENCES messages (id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    model           TEXT NOT NULL,
    dim             INTEGER NOT NULL,
    embedding       vector(1536) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Existing agent fields used: `agents.recent_n_override`,
`agents.top_k_override`. Existing settings fields used:
`memory_defaults.recent_n`, `memory_defaults.top_k`.

### Domain types

```rust
pub struct MemoryBlock {
    pub agent_id: Uuid,
    pub conversation_id: Uuid,
    pub effective_n: i16,
    pub effective_k: i16,
    pub recent: Vec<MemoryTurn>,           // most-recent-N tail, oldest first
    pub retrieved: Vec<RetrievedTurn>,     // top-K by similarity, descending
    pub degraded: bool,                    // true when retrieval skipped (no key / error)
    pub degraded_reason: Option<String>,
}

pub struct MemoryTurn {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub struct RetrievedTurn {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub similarity: f32,                   // cosine similarity, 0.0–1.0
}

pub struct EmbedReport {
    pub embedded: usize,
    pub skipped_already_present: usize,
    pub failed: usize,
}

pub struct ClearReport {
    pub removed: usize,
}
```

## 5. REST API

All responses use the F01 error envelope.

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/conversations/:id/memory/query` | Compose recent-N + top-K for a query |
| POST | `/api/conversations/:id/memory/embed-pending` | Embed any messages missing an embedding |
| DELETE | `/api/conversations/:id/memory` | Clear long-term memory for the conversation |

### POST `/api/conversations/:id/memory/query`

Body:
```json
{ "query": "what did we say about the budget?", "recent_n": 10, "top_k": 5 }
```

`recent_n` and `top_k` are optional. Resolution order:
1. Request body value (if provided)
2. Agent override (`agents.recent_n_override` / `agents.top_k_override`)
3. Global default (`memory_defaults.recent_n` / `memory_defaults.top_k`)

Response:
```json
{
  "conversation_id": "uuid",
  "agent_id": "uuid",
  "effective_n": 10,
  "effective_k": 5,
  "recent": [ { "id":"uuid", "role":"user", "content":"...", "created_at":"..." } ],
  "retrieved": [
    { "id":"uuid", "role":"assistant", "content":"...", "created_at":"...",
      "similarity": 0.87 }
  ],
  "degraded": false,
  "degraded_reason": null
}
```

Degraded responses (no OpenAI key, embedding call failed, or pgvector query
threw) return the same shape with `retrieved: []`, `degraded: true`, and a
short reason string.

### POST `/api/conversations/:id/memory/embed-pending`

Empty body. Response:
```json
{ "embedded": 4, "skipped_already_present": 12, "failed": 0 }
```

Iterates over conversation messages with `role IN ('user','assistant')` and
no matching `message_embeddings` row. Each message is embedded via the
configured provider and inserted; failures increment `failed` and continue.

### DELETE `/api/conversations/:id/memory`

Returns `{ "removed": 17 }`. The `messages` rows are untouched.

### Errors

| Scenario | Status | code |
|---|---|---|
| Conversation not found | 404 | `not_found` |
| `recent_n` or `top_k` out of bounds | 400 | `validation_error` |
| Empty query string on memory/query | 400 | `validation_error` |
| OpenAI key missing | 200 (`degraded:true`) on `query`; 200 with `failed=N` on `embed-pending` | n/a |
| OpenAI provider error | same degrade behavior | n/a |
| DB write/read failure | 500 | `database_error` |

## 6. Validation Rules

| Field | Rule |
|---|---|
| `recent_n` | i16 in `[4, 30]` |
| `top_k` | i16 in `[0, 10]` |
| `query` | 1–4,000 chars after trim |

The same bounds are enforced when an agent override is being saved through
F02's existing AgentUpsert path (`recent_n_override`, `top_k_override`).
F02 already validates these columns; F08 only re-asserts on the memory
query endpoint.

## 7. Frontend

### Per-agent N/K controls

The existing AgentForm already has an Advanced section (per F02 §6). Add
two number inputs there:
- **Recent turns (N)** — placeholder text shows the global default; empty
  → null = use global; in-range positive integer → override.
- **Retrieved turns (K)** — same UX; `0` is allowed and means "no
  semantic retrieval".

Inline validation matches §6 bounds. Save reuses the existing PUT agent
mutation — no new endpoint.

### "Clear long-term memory" action

Add a conversation menu trigger to `ConversationSidebar` (next to the
existing rename/delete) — an "eraser" icon button per row. On click, open
a small confirmation dialog: "Clear long-term memory for this conversation?
This removes the embeddings only; messages remain. Embedded turns: N".
Confirm fires `DELETE /api/conversations/:id/memory`; toast on success
shows `"Cleared N embedded turns"`.

The PRD's "Recalled N earlier turns" indicator under each assistant message
lives in F07 because F07 is what dispatches the chat call. F08 ships only
the data shape via `POST /memory/query`.

## 8. Error Handling

| Scenario | Backend | Frontend |
|---|---|---|
| Out-of-bounds N/K | 400 `validation_error` | Inline error on the AgentForm field |
| Empty query | 400 `validation_error` | F08 itself does not call this from the UI; F07 will surface the error inline in the chat |
| OpenAI key missing on query | 200 with `degraded:true`, `degraded_reason: "no_openai_key"` | F07 renders a "memory unavailable" notice (deferred) |
| OpenAI key missing on embed-pending | 200 with `failed = total_pending` | F07 ignores; embeddings will be retried on the next post-reply call |
| Embedding API 5xx | Same as missing key | Same |
| pgvector retrieval throws | 200 with `degraded:true`, `degraded_reason: "retrieval_error"` | Same |
| Clear-memory DB error | 500 `database_error` | Toast with retry; the action stays available |

The PRD's three F08 error rows map cleanly to the table above.

## 9. Testing Strategy

**Backend integration (`backend/tests/memory.rs`, `sqlx::test` + `mockito`):**
- `embed_pending_creates_rows_idempotently` — first call embeds N, second
  call reports `skipped_already_present = N` and `embedded = 0`
- `embed_pending_skips_system_messages`
- `embed_pending_reports_failure_count_when_provider_500s` — `mockito`
  serves 500 for one batch; the row count grows by the successful subset
- `query_returns_recent_window_only_when_k_is_zero`
- `query_returns_top_k_excluding_recent_window` — seed 20 messages, embed
  all, query with `recent_n=4 top_k=3`, assert the 3 retrieved ids are
  outside the most-recent-4 tail
- `query_resolves_overrides_then_globals` — set agent overrides, omit
  `recent_n` in the body, assert `effective_n` equals the override
- `query_validates_bounds` — `recent_n=2` → 400; `top_k=11` → 400
- `query_degrades_when_no_openai_key` — store empty key, expect
  `degraded:true` and `retrieved: []`
- `clear_removes_embeddings_only` — embed 5, DELETE, assert
  `message_embeddings` count is 0 and `messages` count unchanged
- `query_404_for_missing_conversation`

**Frontend (Vitest + Testing Library):**
- AgentForm renders the Advanced section with N and K inputs; saving a
  valid override calls PUT with the right body; out-of-bounds shows
  inline error
- `ConversationSidebar` clear-memory icon opens the confirm dialog;
  confirming fires DELETE and toasts success
- Mocked DELETE returning 500 toasts an error and leaves the action
  available

**Manual smoke (documented in `verify.md` after Phase 4):**
- Save an OpenAI key in Settings
- Seed an agent + a conversation with ≥ 20 user/assistant messages (via
  psql)
- `POST /api/conversations/:id/memory/embed-pending` — confirm
  `embedded > 0`; running it again returns `embedded = 0`
- `POST /api/conversations/:id/memory/query` with a relevant query —
  assert `retrieved` contains turns from outside the recent window with
  positive similarity scores
- Open the chat surface → use the eraser icon on a conversation → confirm
  the embeddings count drops to 0 (via psql) and a fresh
  `embed-pending` re-creates them
- Remove the OpenAI key and query again — response is `degraded:true`

## 10. Acceptance Criteria Mapping

PRD §9 F08 → spec sections:

| Criterion | Section |
|---|---|
| Last N (default 10) turns included verbatim | §5 query response.recent; §9 test `query_returns_recent_window_only_when_k_is_zero` |
| Older turns embedded after assistant reply completes | §2 assumption #6 + §5 embed-pending (F07 wires the trigger) |
| Top-K (default 5) similar turns retrieved | §5 query response.retrieved; §9 `query_returns_top_k_excluding_recent_window` |
| "Recalled N earlier turns" indicator | Deferred to F07 — F08 returns the data |
| Clear long-term memory removes embeddings only | §5 DELETE; §9 `clear_removes_embeddings_only` |
| Vector retrieval fallback when failing | §5 degraded flag; §9 `query_degrades_when_no_openai_key` |
| N ∈ [4,30], K ∈ [0,10] bounded and configurable per agent | §6; §7 AgentForm; §9 `query_validates_bounds`, `query_resolves_overrides_then_globals` |

Cross-feature criteria from PRD §9 covered here:
- "The recent-N verbatim turns and top-K retrieved turns provided by F08
  are included in the F07 request and visibly attributed in the UI" — §5
  query contract is the consumed surface; F07 will read it and render the
  indicator.
