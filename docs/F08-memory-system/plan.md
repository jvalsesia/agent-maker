# F08. Memory System — Implementation Plan

## Prerequisites

- F01–F06 are on `main` (`docker compose up -d`, `cargo run`, `pnpm dev`)
- `message_embeddings` table and HNSW index already provisioned in
  `0002_pgvector.sql` — no new migration
- F02 `agents.recent_n_override` and `agents.top_k_override` columns
  already exist (added in F01); F01 settings `memory_defaults.recent_n`
  and `memory_defaults.top_k` are persisted and validated

## Phase 1 — Embedding Provider and Memory Service

1. **Module scaffold (`backend/src/memory/`)** — Create `mod.rs`,
   `model.rs`, `service.rs`, `embedding.rs`. Wire `pub mod memory;` from
   `lib.rs` next to `agents`, `conversations`, etc. Mirror the F03/F06
   module shape.

2. **`EmbeddingProvider` trait + OpenAI impl (`embedding.rs`)** — Tiny
   trait `async fn embed_one(&self, text: &str) -> Result<Vec<f32>,
   EmbeddingError>`. Implementation hits
   `https://api.openai.com/v1/embeddings` with model
   `text-embedding-3-small`, honoring `OPENAI_API_BASE` for tests
   (mirrors the existing `openai.rs` chat client). Returns a typed
   `NoKey` variant when the secret store has no `openai` entry so the
   service can degrade gracefully.

3. **Domain types (`model.rs`)** — `MemoryBlock`, `MemoryTurn`,
   `RetrievedTurn`, `EmbedReport`, `ClearReport`, plus request bodies
   `MemoryQuery { query, recent_n?, top_k? }` and the
   `Warning` envelope shape reused from other modules.

4. **`MemoryService` (`service.rs`)** — Constructor takes
   `PgPool + Arc<dyn EmbeddingProvider + Send + Sync>`. Methods:
   - `resolve_n_k(conversation_id, override_n, override_k)` — looks up
     the parent agent, merges agent overrides + global defaults, applies
     the per-request override last; enforces `N ∈ [4,30]`, `K ∈ [0,10]`.
   - `embed_pending(conversation_id)` — selects every user/assistant
     message that lacks a row in `message_embeddings`; embeds each
     individually with try/catch; counts embedded/skipped/failed; commits
     each successful embedding immediately (no batch TX).
   - `query(conversation_id, body)` — fetches the recent-N tail
     (excluding `system`), embeds the user query, and runs a
     `<#>`-distance pgvector query for the top-K most similar
     embeddings whose `message_id` is **not** in the recent-N tail.
     Returns `degraded=true` when the embedding step fails or when K=0
     the retrieval step is skipped (without setting `degraded`).
   - `clear(conversation_id)` — `DELETE FROM message_embeddings WHERE
     conversation_id = $1`, returning the `rows_affected` count.

5. **Wire into `AppState`** — `build_app` / `build_app_with_store`
   instantiate the OpenAI embedding provider from the shared secrets
   store and pass it into `MemoryService`. Add a
   `memory: MemoryService` field to `AppState`.

## Phase 2 — Backend REST Surface

6. **`routes/memory.rs`** — Three handlers per spec §5:
   `POST /api/conversations/:id/memory/query`,
   `POST /api/conversations/:id/memory/embed-pending`,
   `DELETE /api/conversations/:id/memory`. Validate conversation
   existence at the route layer (return 404 before calling the service).
   Map `AppError::Validation` → 400 with the F01 envelope.

7. **Wire routes** — Add `.merge(memory::routes())` to the `/api` nest in
   `routes/mod.rs`.

8. **Integration tests (`backend/tests/memory.rs`)** — Cover the ten
   cases enumerated in spec §9 using `sqlx::test` for Postgres and
   `mockito` for the OpenAI HTTP surface (set `OPENAI_API_BASE` to the
   mock URL inside an `ENV_LOCK`, the same pattern as
   `tests/integration.rs::test_provider_test_invalid_key`). Each test
   inserts the agent + conversation + messages directly and exercises
   one path end-to-end.

## Phase 3 — Frontend

9. **REST client (extend `lib/api.ts`)** — Strongly typed
   `MemoryBlock`/`RetrievedTurn`/`EmbedReport`/`ClearReport` and three
   wrappers `api.queryMemory`, `api.embedPending`, `api.clearMemory`.

10. **Hooks (`hooks/useMemory.ts`)** — `useClearMemory(conversationId,
    agentId)` and `useEmbedPending(conversationId)` mutations.
    `clearMemory` invalidates nothing globally — F07 will own the
    "memory indicator" cache. (No query hook in F08; F07 owns the
    query consumer.)

11. **Per-agent N/K controls in AgentForm** — Extend the existing
    Advanced section with `Recent turns (N)` and `Retrieved turns (K)`
    number inputs. Empty input = null override = use global default;
    placeholder text shows the global value pulled from `useSettings`.
    Reuse the existing PUT agent mutation (no new endpoint).

12. **Clear-memory action in ConversationSidebar** — Add an eraser-icon
    button next to the existing pencil/trash; clicking opens a small
    `ClearMemoryDialog` with the embedded-turn count (fetched lazily on
    open — keeps the sidebar render path cheap). Confirm fires DELETE
    and toasts the result.

## Phase 4 — Verification

13. **Frontend tests (Vitest + Testing Library)** — AgentForm N/K
    validation + save round-trip; ConversationSidebar clear-memory
    confirm + success toast; mocked DELETE 500 toasts an error and
    leaves the row intact. Reuse the `mockFetch` harness from
    `src/test-utils.tsx`.

14. **Manual end-to-end smoke** — Run the full stack with an OpenAI key
    saved. Walk through the spec §9 smoke list: seed messages via psql,
    embed-pending (twice, to assert idempotency), query, clear via the
    chat sidebar, remove the key and confirm degraded responses. Capture
    the steps in `docs/F08-memory-system/verify.md`.
