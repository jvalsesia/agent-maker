# F06. Conversation Management — Implementation Plan

## Prerequisites

- F01–F05 are on `main` (`docker compose up -d`, `cargo run`, `pnpm dev`)
- `conversations` and `messages` tables already exist (F01 migration
  `0001_init.sql`) — no new migration is required
- F02 `agents` table is the FK target for `conversations.agent_id`

## Phase 1 — Backend Service

1. **Module scaffold (`backend/src/conversations/`)** — Create `mod.rs`,
   `model.rs`, `service.rs`. Wire `pub mod conversations;` from `lib.rs`
   next to `agents`, `skills`, `skill_attachments`, and `templates`. Mirror
   the existing module shape and re-export `ConversationsService` from
   `mod.rs`.

2. **Domain types (`model.rs`)** — `Conversation` (matches the existing
   table), `Message`, request DTOs (`CreateInput { title: Option<String> }`,
   `RenameInput { title: String }`), response envelopes
   (`ConversationsList`, `ConversationResponse`, `ListMessagesResponse`),
   and a `Warning` struct reused from the F02/F03/F04 envelope shape.

3. **`ConversationsService`** — `list_for_agent(agent_id)` with
   auto-create semantics: a single transaction selects existing rows; if
   the count is zero, inserts a placeholder `("New conversation")` and
   returns the singleton; results sorted by `last_activity_at DESC`. Plus
   `create`, `get`, `rename` (updates `title` and bumps
   `last_activity_at`), `delete` (relies on FK cascade), and
   `list_messages(conv_id)` returning `Vec<Message>` ordered ASC. All
   methods validate that the parent agent/conversation exists and return
   `AppError::NotFound` otherwise.

4. **Wire into `AppState`** — Add `conversations: ConversationsService`
   to `routes::mod::AppState`; instantiate it from `build_app` /
   `build_app_with_store` in `lib.rs`.

## Phase 2 — Backend REST Surface

5. **`routes/conversations.rs`** — Mount the six handlers per spec §5
   under `/api/agents/:agent_id/conversations` and `/api/conversations/:id`.
   Reuse the existing `AppResult<Json<_>>` pattern; map service errors via
   `AppError` (validation → 400, not-found → 404, DB → 500).

6. **Wire routes** — Add `.merge(conversations::routes())` to the `/api`
   nest in `routes/mod.rs` alongside the existing entries.

7. **Integration tests (`backend/tests/conversations.rs`)** — Cover the
   nine cases enumerated in spec §8: auto-create singleton on first list,
   stability across repeated lists, sort by last-activity, create with and
   without title, rename success + validation rejects, 404 on missing
   conversation, cascade delete of messages, message-list ordering, 404 on
   list messages for missing conversation. Use `sqlx::test` per the F02–
   F05 harness.

## Phase 3 — Frontend

8. **REST client (extend `lib/api.ts`)** — Add strongly typed
   `Conversation` and `Message` interfaces plus `api.listConversations`,
   `api.createConversation`, `api.renameConversation`,
   `api.deleteConversation`, and `api.listMessages` wrappers. Reuse the
   existing `request<T>` + `ApiError` plumbing.

9. **React Query hooks (`hooks/useConversations.ts`)** —
   `useConversations(agentId)`, `useMessages(conversationId)`, plus
   `useCreateConversation`, `useRenameConversation`,
   `useDeleteConversation`. Mutations invalidate
   `['conversations', agentId]`; delete additionally clears
   `['messages', id]` cache.

10. **Draft store (`hooks/useDrafts.ts`)** — A React context provider with
    `Map<conversationId, string>` in state. Exposes `getDraft` /
    `setDraft`. Mounted by `ChatPage` so the lifetime matches the route.

11. **`ChatPage.tsx` + sidebar + message list** — New
    `pages/Chat/ChatPage.tsx` route surface implementing the two-pane
    layout from spec §6: `ConversationSidebar` (list, create button,
    inline rename, delete dialog) + `MessageList` (read-only bubbles with
    the F07 placeholder copy when empty) + disabled draft textarea wired
    to the drafts hook. Active conversation id is local state, defaulting
    to the first item in the sorted list. Include a
    `DeleteConversationDialog` mirroring the F03 delete-skill pattern.

12. **Routing and entry points** — Add
    `<Route path="/agents/:id/chat" element={<ChatPage />} />` to
    `router.tsx`. Add `MessageSquare` chat-icon buttons to `AgentsList`
    rows and to the `AgentForm` header (edit mode only) that navigate to
    the chat route.

## Phase 4 — Verification

13. **Frontend tests** — Add Vitest + Testing Library cases per spec §8
    under `pages/Chat/`: sidebar renders the auto-created singleton,
    create-conversation flow, inline rename happy path, delete dialog
    with message count, draft preservation across switches, message-list
    rendering and empty-state placeholder. Reuse `mockFetch` /
    `jsonResponse` from `src/test-utils.tsx`.

14. **Manual end-to-end smoke** — Run the full stack against a clean
    workspace; click "Open chat" on an existing agent and walk through
    the spec §8 smoke list (auto-create, multi-create, draft
    preservation, rename persistence, delete-then-switch, manual
    `messages` INSERT round-trip). Capture the steps in
    `docs/F06-conversation-management/verify.md`.
