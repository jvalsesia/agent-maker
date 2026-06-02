# F11. Sub-agents — Implementation Plan

## Prerequisites

- F02 (Agent Management), F04 (Skill Attachment), and F07 (Chat Runtime) are
  implemented on `main` — all three are dependencies and supply the patterns reused here.
- PostgreSQL + pgvector running via `docker compose up -d`; SQLx migrations apply on
  startup.
- Read `spec.md` (this folder) before each phase. Backend follows the per-domain
  `model.rs` / `service.rs` / `mod.rs` + `routes/` split; frontend server state goes
  through TanStack Query hooks; all user-facing strings live in both `en.json` and
  `pt-BR.json`.
- Use the `rust-best-practices` skill while writing backend code and
  `vite-react-best-practices` for the frontend.

## Phase 1 — Backend: data model & sub-agent attachment domain

1. **Migration** — Add `backend/migrations/0007_subagents.sql` creating the
   `agent_subagents` ordered M:N table (parent/child FKs with cascade, alias, description,
   `position 0..=9`, the unique and self-attach constraints) and altering `messages` with
   the nullable `subagent_alias` / `subagent_agent_id` columns, per the spec Data Model.

2. **Domain module** — Create `backend/src/subagents/{model.rs,service.rs,mod.rs}`
   modeled on `skill_attachments`. Implement `list`, `attach`, `update`, `reorder`, and
   `detach` on `SubagentsService`, including alias format/uniqueness validation, the
   `0..=10` cap, self-attach rejection, DFS-based cycle detection over existing
   parent→child edges, and gap-closing position renumber on detach.

3. **Routes & wiring** — Add `backend/src/routes/subagents.rs` exposing the endpoints in
   the spec under `/api/agents/{agent_id}/subagents`. Register the module, add
   `subagents: SubagentsService` to `AppState`, and merge the routes into the protected
   router in `routes/mod.rs`; construct the service in `main.rs`.

4. **Unit tests** — In `service.rs` `#[cfg(test)]`, cover self-attach, the cap, cycle vs
   valid DAG (diamond), alias format + duplicates, default-alias slug, reorder, and
   detach renumber. Run `cargo test` + `cargo clippy --all-targets`.

## Phase 2 — Backend: chat delegation runtime

1. **Message persistence** — Extend `conversations::model::Message` with the two new
   fields and add `NewMessage::subagent(...)`; update `ConversationsService` insert/select
   SQL to carry `subagent_alias` / `subagent_agent_id`.

2. **Mention parsing** — Add `parse_mentions(text, &attached)` to `chat::service`
   returning the ordered, de-duplicated, ≤3 matched sub-agents, the child task (matched
   handles stripped, unmatched left literal), and an overflow flag. Cover with
   `#[cfg(test)]` unit tests.

3. **Blocking child call** — Add `run_subagent_blocking` that composes the child's own
   system prompt (`AttachmentsService::compose` + F09 language directive), builds a
   single-user-turn `ChatRequest` with the child's own provider/model/key, and drains the
   provider stream to a `String`. Inject `SubagentsService` into `ChatService`.

4. **Delegation branch & SSE** — Add the `StreamEvent::Subagent` variant
   (`chat/model.rs`) and the "Sub-agent responses" block to `chat/compose.rs`. In
   `start()` parse mentions and pass them to `stream_task`; in the task, when mentions are
   present, run each child blocking, persist + emit a `subagent` event per child, halt on
   child error (persist error turn, emit `Error`, skip parent), otherwise augment the
   parent `base_system` with the responses block and run the existing parent stream.

5. **Integration tests** — Add `backend/tests/subagents.rs` (CRUD + cycle 422 + reorder)
   and `backend/tests/chat_delegation.rs` (mention → `subagent` frame + persisted
   `subagent_alias` row + parent turn with the context block; child-error holds parent;
   unmatched mention completes normally), using the existing stub-provider test harness.

## Phase 3 — Frontend: sub-agent management UI

1. **API & hooks** — Add the subagent types and `listSubagents` / `attachSubagent` /
   `updateSubagent` / `reorderSubagents` / `detachSubagent` to `src/lib/api.ts`, and the
   new `Message` fields. Create `src/hooks/useSubagents.ts` mirroring `useAgentSkills`
   with query-invalidation on each mutation.

2. **Sub-agents section** — Build `src/pages/Agents/SubagentsSection.tsx` (parallel to the
   Skills section): list attached sub-agents with `@handle`, child name, and hint; drag
   reorder; detach; alias/description inline edit. Mount it on the agent detail page.

3. **Picker & quick-create** — Build `src/pages/Agents/SubagentPicker.tsx` listing other
   agents (search + select), disabling the parent itself and any agent that would form a
   cycle (with a tooltip). Add the **"New sub-agent"** action that opens the standard
   agent-create form and, on save, attaches the new agent to the parent.

4. **i18n + tests** — Add all new keys to `en.json` and `pt-BR.json`. Add
   `useSubagents.test` and `SubagentsSection.test`. Run `pnpm typecheck` + `pnpm test`.

## Phase 4 — Frontend: chat delegation UX & end-to-end

1. **SSE handling** — Extend the `src/lib/api.ts` chat event union with `subagent` and
   update `useChat` to accumulate sub-agent events into labeled draft bubbles rendered
   before the parent draft; reconcile against canonical rows on settle.

2. **Labeled bubble** — Update `MessageBubble.tsx` (+ `MessageList.tsx`) to render the
   sub-agent variant when `subagent_alias` is set (label = child name/alias, visually
   subordinate), and show the "consulted N sub-agents" chip on the parent reply.

3. **Composer autocomplete** — Add `@`-autocomplete to `Composer.tsx` sourcing the
   parent agent's attached aliases; insert the selected `@handle` on choose.

4. **i18n + tests + verify** — Localize all new chat strings in both catalogs. Add
   `Composer`, `MessageBubble`, and `useChat` tests for the delegation path. Run the full
   frontend suite and `cargo test`, then use the `verify` skill to exercise an end-to-end
   `@mention` delegation against the running stack.
