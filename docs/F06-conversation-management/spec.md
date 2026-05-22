# F06. Conversation Management — Technical Specification

## 1. Overview

F06 turns the `conversations` and `messages` tables that F01 already
provisioned into a usable feature: per-agent conversation threads with auto-
creation on first open, CRUD operations (new, rename, delete, switch),
last-activity ordering, and a chat-surface shell that the F07 streaming
runtime will plug into later. The feature ships REST endpoints for both
conversations and read-only message listing so the frontend can hydrate the
chat shell, plus a new `/agents/:id/chat` route with a left sidebar of
conversations and a main pane that draws message history. In-memory draft
state preserves any unsent text when the user switches between conversations
within the same session.

## 2. Scope

**Included:**
- `conversations::service` with `list_for_agent`, `get`, `create`,
  `rename`, `delete`, plus `list_messages(conversation_id)` reading from the
  existing `messages` table
- REST surface under `/api/agents/:agent_id/conversations*` and
  `/api/conversations/:id*` per spec §5
- Auto-creation policy: opening an agent's chat surface with no existing
  conversations creates one transparently (server-side helper invoked from
  the list endpoint when the agent has zero conversations)
- Frontend `/agents/:id/chat` route with conversation sidebar, conversation
  switcher, rename (inline edit), delete (confirm modal showing message
  count), and a chat shell that hydrates messages via the new GET endpoint
- In-memory draft persistence keyed by conversation id (survives switching,
  not reload)
- A "Chat" entry point from the agents list (a chat-icon action per row) and
  from the agent edit form ("Open chat" button)

**Integrated from PRD blocks:**
- `Consumes`: F01 persistence layer (existing `conversations` + `messages`
  tables); F02 agent definitions (FK + agent-scoped routes)
- `Provides`: conversation containers with message history; current
  conversation selection per agent (consumed by F07 and F08 in later waves)
- `Capabilities`: 1..N threads per agent, auto-generated title from first
  user message truncated at 60 chars, ops new/rename/delete/switch,
  auto-create on first open, list sorted by `last_activity_at DESC`, no hard
  limit
- `Experience`: left sidebar listing conversations, "+ New conversation" at
  top, active highlight, unsent drafts preserved on switch, rename via inline
  edit, delete confirmation showing message count
- `Error Handling`: rename/delete failure surfaces inline retry/toast;
  conversation remains visible

**Deferred (out of F06):**
- Composer/Send flow and streaming responses — F07
- Title auto-fill from the first user message body — F07 (when the first
  POST message lands, the runtime sets `title`; F06 only ships the storage
  + the rename UX). Newly-created conversations get the placeholder title
  `"New conversation"`.
- Memory recall indicator / embedding-aware behavior — F08
- Per-agent advanced settings exposing N/K — F08

**Assumptions / decisions (PRD did not specify):**
1. **Tables already exist.** `conversations` and `messages` were added in
   migration `0001_init.sql`. F06 ships no new migration; it only consumes
   the existing schema.
2. **Routing.** A new `/agents/:id/chat` route hosts the chat surface. The
   existing `/agents/:id` route continues to be the edit form. The chat
   route URL does not include the conversation id — the active conversation
   is tracked in component state and persisted only across in-session
   switches.
3. **Auto-create policy.** `GET /api/agents/:agent_id/conversations` returns
   the list; if the agent has zero conversations, the server creates a
   placeholder titled `"New conversation"` in the same transaction and
   returns the singleton list. This keeps the "auto-create on first open"
   semantics deterministic without a separate ensure endpoint.
4. **Title default.** New conversations are created with
   `title = "New conversation"`. F07 will overwrite the title from the first
   user message (truncated to 60 chars) on send.
5. **Messages endpoint is read-only in F06.** `GET
   /api/conversations/:id/messages` returns the persisted history ordered
   by `created_at ASC`. POST and streaming arrive with F07.
6. **Drafts are in-memory only.** A React context (or simple hook) keyed by
   conversation id holds the draft string; switching conversations preserves
   the prior draft within the session. Reload clears drafts — PRD does not
   require otherwise.
7. **Soft delete is out of scope.** Delete is hard delete; `messages` is
   `ON DELETE CASCADE` per the F01 schema, so all child messages are removed.

## 3. Component Overview

### Repository additions

```
/backend
└── /src
    ├── conversations/
    │   ├── mod.rs
    │   ├── model.rs                 # Conversation, ConversationSummary,
    │   │                            # CreateInput, RenameInput, Message,
    │   │                            # ListMessagesResponse, Warnings
    │   └── service.rs               # ConversationsService
    └── routes/
        └── conversations.rs         # mount /api/agents/:id/conversations*
                                     # and /api/conversations/:id*

/backend/tests
└── conversations.rs                 # integration tests (sqlx::test)

/frontend
└── /src
    ├── pages/
    │   └── Chat/
    │       ├── ChatPage.tsx         # /agents/:id/chat route surface
    │       ├── ConversationSidebar.tsx
    │       ├── MessageList.tsx
    │       ├── DeleteConversationDialog.tsx
    │       └── index.ts
    ├── hooks/
    │   ├── useConversations.ts
    │   └── useDrafts.ts             # in-memory draft store (per session)
    └── lib/
        └── api.ts                   # extended with conversations + messages
```

### Backend wiring

- `lib.rs::build_app` and `build_app_with_store` instantiate
  `ConversationsService::new(pool.clone())` and add it to `AppState` next to
  `agents`, `skills`, `attachments`, and `templates`.
- `routes/mod.rs` merges `conversations::routes()` into the `/api` nest.

### Frontend wiring

- `router.tsx` adds `<Route path="/agents/:id/chat" element={<ChatPage />} />`.
- `NavSidebar` is unchanged (Conversations is a per-agent surface, not a top-
  level nav entry).
- `AgentsList` rows and `AgentForm` header get a `MessageSquare` action that
  navigates to `/agents/:id/chat`.

## 4. Data Model

F06 adds no migrations. The relevant tables already exist (see
`backend/migrations/0001_init.sql`):

```sql
CREATE TABLE conversations (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id         UUID NOT NULL REFERENCES agents (id) ON DELETE CASCADE,
    title            TEXT NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    message_count    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_conversations_agent_activity
    ON conversations (agent_id, last_activity_at DESC);

CREATE TABLE messages (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES conversations (id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
    content         TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'complete'
                    CHECK (status IN ('complete','stopped','error')),
    model           TEXT,
    token_count     INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_messages_conversation_created
    ON messages (conversation_id, created_at);
```

### Domain types

```rust
pub struct Conversation {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub message_count: i32,
}

pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,           // 'user' | 'assistant' | 'system'
    pub content: String,
    pub status: String,         // 'complete' | 'stopped' | 'error'
    pub model: Option<String>,
    pub token_count: Option<i32>,
    pub created_at: DateTime<Utc>,
}
```

### Validation rules

| Field | Rule | On violation |
|---|---|---|
| `title` (create/rename) | 1–80 chars, trimmed | 400 `validation_error`, field=`title` |
| `agent_id` (path) | must exist | 404 `not_found` |
| `conversation_id` (path) | must exist | 404 `not_found` |

PRD says the auto-generated title is truncated at 60 chars; the column is
soft-capped at 80 to give user renames a little headroom.

## 5. REST API

All responses use the F01 JSON error envelope. All bodies are JSON.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/agents/:agent_id/conversations` | List agent's conversations (auto-create singleton when empty) |
| POST | `/api/agents/:agent_id/conversations` | Create a new conversation (optional `title`) |
| GET | `/api/conversations/:id` | Get one conversation |
| PATCH | `/api/conversations/:id` | Rename (body: `{ "title": "..." }`) |
| DELETE | `/api/conversations/:id` | Delete (hard; cascades to messages) |
| GET | `/api/conversations/:id/messages` | List messages, ordered by `created_at ASC` |

### GET `/api/agents/:agent_id/conversations`

```json
{
  "conversations": [
    {
      "id": "uuid",
      "agent_id": "uuid",
      "title": "New conversation",
      "created_at": "...",
      "last_activity_at": "...",
      "message_count": 0
    }
  ]
}
```

Sorted by `last_activity_at DESC`. If the agent has zero conversations on
this call, the server creates a singleton placeholder
(`title = "New conversation"`) inside a transaction and returns the
one-element list.

### POST `/api/agents/:agent_id/conversations`

Optional body: `{ "title": "Optional starting title" }`. Returns:

```json
{
  "conversation": { ...Conversation... },
  "warnings": []
}
```

Default `title = "New conversation"`.

### PATCH `/api/conversations/:id`

```json
{ "title": "Renamed thread" }
```

Returns `{ "conversation": {...}, "warnings": [] }`. Bumps
`last_activity_at` only on title change.

### DELETE `/api/conversations/:id`

Returns `204 No Content`. Cascade removes messages via FK.

### GET `/api/conversations/:id/messages`

```json
{
  "conversation": { ...Conversation... },
  "messages": [
    { "id": "uuid", "role": "user", "content": "...", "status": "complete",
      "model": null, "token_count": null, "created_at": "..." }
  ]
}
```

### Errors

| Scenario | Status | code |
|---|---|---|
| Agent not found | 404 | `not_found` |
| Conversation not found | 404 | `not_found` |
| Empty/over-long title | 400 | `validation_error` |
| DB write failure | 500 | `database_error` |

## 6. Frontend

### Routes

- `/agents/:id/chat` → `ChatPage`. Layout: left sidebar (~260px) listing
  conversations + new-conversation button; main pane shows message history
  and a draft input placeholder (composer ships in F07).

### `ChatPage.tsx`

- On mount, calls `GET /api/agents/:id/conversations`. The server-side auto-
  create ensures the response is never empty; the page selects the first
  result as active.
- Active conversation id is local state. URL stays `/agents/:id/chat`; no
  per-conversation deep link in F06.
- When the active conversation changes, `MessageList` fetches messages and
  the composer textarea swaps its `value` to the draft stored under the new
  conversation id (`useDrafts(activeId)`).

### `ConversationSidebar.tsx`

- Renders sorted list (already sorted server-side; no client re-sort).
- Each row: title, last-activity relative time, message-count badge. Active
  row highlighted.
- Hover reveals a pencil-icon (rename) and trash-icon (delete) button.
- "+ New conversation" button at the top calls the create mutation and
  selects the new conversation on success.
- Inline rename: clicking pencil swaps the title to an `<input>`; Enter
  saves (PATCH), Escape cancels.
- Delete opens `DeleteConversationDialog` which shows the message count and
  a `Delete` / `Cancel` pair. On confirm the mutation runs; on success the
  next conversation in the sorted list becomes active.

### `MessageList.tsx`

- Read-only message list in F06. Each message is a simple bubble
  (`role = user` right-aligned, `assistant`/`system` left-aligned, monospace
  for code-fenced content). Empty-state copy when `messages.length === 0`:
  "No messages yet. Send your first message once the chat runtime ships
  (F07)."
- No composer or send button in F06 — a disabled placeholder textarea sits
  at the bottom with a tooltip explaining the F07 dependency.

### `useDrafts.ts`

- A React context provider mounted by `ChatPage`. Holds a
  `Map<conversationId, string>` in state. Exposes `getDraft(id)` and
  `setDraft(id, value)`. No persistence layer.

### `useConversations.ts`

- `useConversations(agentId)` → `useQuery(['conversations', agentId])`.
- `useMessages(conversationId)` → enabled when `conversationId` is set.
- Mutations: `useCreateConversation`, `useRenameConversation`,
  `useDeleteConversation`. All invalidate `['conversations', agentId]`;
  delete additionally invalidates `['messages', deletedId]` to clear cache.

### Entry points

- `AgentsList`: add a `MessageSquare` ghost button per row that navigates to
  `/agents/:id/chat`.
- `AgentForm`: when editing an existing agent, add an "Open chat" button to
  the header that does the same.

## 7. Error Handling

| Scenario | Backend | Frontend |
|---|---|---|
| Validation (title empty/over-long) | 400 `validation_error`, field=`title` | Inline error on the rename input; original title restored on cancel |
| Agent not found | 404 `not_found` | Redirect `/agents/:id/chat` → `/agents` with toast "Agent no longer exists" |
| Conversation not found | 404 `not_found` | Remove from local list; toast |
| DB error on create | 500 `database_error` | Toast "Couldn't create conversation — please try again"; sidebar stays as-is |
| DB error on rename | 500 `database_error` | Inline error on the input; previous title restored |
| DB error on delete | 500 `database_error` | Toast with retry; conversation remains visible |
| Messages fetch fails | 500 `database_error` | `MessageList` shows error block with retry button; conversation list unaffected |

The PRD entries — "save fails after a user message is sent" and "delete
fails: toast with retry" — map to the create/delete rows above. The
message-send error path itself lives in F07.

## 8. Testing Strategy

**Backend integration (`backend/tests/conversations.rs`, sqlx::test):**
- `list_auto_creates_singleton_when_agent_empty` — first GET against an
  agent with zero conversations returns one placeholder titled "New
  conversation"; subsequent GETs return the same single conversation
  (no duplicate auto-creation)
- `list_returns_sorted_by_last_activity_desc`
- `create_appends_new_conversation` with default title and explicit title
- `rename_updates_title_and_last_activity` — also asserts
  `last_activity_at` moved forward
- `rename_rejects_empty_and_overlong_title` (validation 400)
- `get_returns_404_for_missing`
- `delete_cascades_messages` — seed messages directly, delete the
  conversation, assert messages row count is 0
- `list_messages_orders_by_created_at_asc` — seed three messages, fetch,
  assert order
- `list_messages_returns_404_for_missing_conversation`

**Frontend (Vitest + Testing Library):**
- `ChatPage` renders the sidebar with the auto-created singleton when the
  mocked list returns one conversation
- Clicking "+ New conversation" calls the create mutation and selects the
  new conversation
- Inline rename: clicking the pencil reveals an input, typing + Enter
  triggers the PATCH and updates the title in place
- Delete dialog shows the message count; confirming triggers DELETE and
  removes the row from the list
- Switching conversations preserves the draft text — the draft hook stores
  text under the previous conv id and restores it when the user switches
  back
- `MessageList` renders user/assistant bubbles in order; empty state shows
  the F07 placeholder copy

**Manual smoke (documented in `verify.md` after Phase 4):**
- From an existing agent, click "Open chat" → land on `/agents/:id/chat` →
  sidebar shows one auto-created conversation
- Click "+ New conversation" twice → three conversations listed, newest at
  top
- Type into the placeholder composer → switch to another conversation →
  type a different draft → switch back → original draft is restored
- Rename the active conversation → reload → renamed title persists
- Delete the active conversation → next conversation becomes active; the
  deleted one is gone
- Manually `INSERT` a `messages` row via psql → reload → `MessageList`
  renders the bubble

## 9. Acceptance Criteria Mapping

PRD §9 F06 → spec sections:

| Criterion | Section |
|---|---|
| Opening an agent for the first time auto-creates an initial conversation | §5 GET list auto-create; §8 `list_auto_creates_singleton_when_agent_empty` |
| Start new, switch, state preserved | §6 ChatPage; §8 draft preservation test |
| Auto-title from first user message, truncated 60 chars | Deferred to F07 — F06 ships the storage + rename UX (§2 assumptions) |
| Rename persists | §5 PATCH; §8 rename test |
| Delete confirmation shows message count | §6 DeleteConversationDialog; §8 dialog test |
| Conversations list sorted by last-activity desc | §4 index; §5 GET list; §8 sort test |

Cross-feature criteria from PRD §9 covered here:
- "Conversations created in F06 expose their full message history to F08
  for embedding and retrieval, and to F07 for prompt composition" — §5
  GET `/api/conversations/:id/messages` is the consumed contract; F07 and
  F08 will add the write paths in later waves.
