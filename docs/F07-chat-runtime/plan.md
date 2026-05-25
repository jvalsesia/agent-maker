# Implementation Plan: Chat Runtime

**Prerequisites:**
- Rust backend with Axum, `sqlx` (PostgreSQL), `async-trait`, `tokio`, and an
  async HTTP client already wired for the LLM providers (F01).
- Frontend: React + Vite + TypeScript, `@tanstack/react-query`; add
  `react-markdown`, `remark-gfm`, `rehype-highlight` (+ a highlight.js theme).
- Implemented dependencies present: F02 agents, F04 skill attachment `compose`,
  F06 conversations/messages, F08 `MemoryService`, F01 provider registry.
- Local PostgreSQL reachable per F01 (`docker compose up -d`).

## Stage 1: Provider Streaming Foundation

**1. Streaming provider contract** - Extend the `LlmProvider` trait with a streaming chat method and define the shared chat request/message/chunk/usage/error types. Reference spec §4 and §5 for the event shapes and the success/failure split between preflight HTTP errors and mid-stream events.

**2. Provider implementations** - Implement the streaming chat method for the Anthropic, OpenAI, and OpenAI-compatible providers, mapping the composed request to each provider's streaming API and surfacing token usage when the provider reports it, with a fallback otherwise.

## Stage 2: Persistence and Composition

**3. Recall and message schema migration** - Add the migration introducing the recalled-turn attribution table and the new message finish-reason column, following the existing migrations layout and cascade conventions described in spec §6.

**4. Conversation persistence helpers** - Extend the conversation service with helpers to insert and update messages, supersede the trailing assistant turn for retry, and write and read recalled-turn references. Extend the existing messages listing so assistant messages expose their recalled turns and finish reason per spec §5.

**5. Prompt composer** - Build the composition layer that assembles the system block from the agent prompt, ordered skill bodies, and the labeled "Earlier relevant context" memory section, then forms the message array from the recent conversation tail and the current user message. Include the context-fit reduction that lowers retrieved turns and trims until the prompt fits the model budget, per spec §3 and §8.

## Stage 3: Chat Service and SSE Endpoint

**6. Chat service orchestration** - Create the chat service that resolves the agent and effective memory parameters, queries F08 for the memory block, composes the prompt, dispatches the provider stream, persists the user and assistant messages and their recalls, and spawns the asynchronous embedding of older turns after the reply completes. Reference the degraded-memory and overflow behaviors in spec §1 and §5.

**7. SSE chat endpoint** - Add the streaming chat route that emits the meta, chunk, done, and error events, enforces one in-flight request per conversation, handles client disconnect by persisting the partial reply as stopped, and returns pre-stream errors using the existing error response shape. Register it in the router and add the chat service to application state per spec §2 and §4.

## Stage 4: Frontend Streaming Client

**8. Streaming API client and hook** - Add the chat streaming generator to the API client that issues the POST, reads the response body, parses SSE frames, and honors an abort signal, plus a chat hook that accumulates chunks into the in-flight assistant message and exposes stop and retry. Reference spec §5 for the event contract.

**9. Composer and send flow** - Replace the placeholder composer with an input supporting Enter-to-send, Shift+Enter newlines, focus shortcut, and the in-flight send guard with its inline notice, wired to the chat hook and existing draft state per spec §4 Experience.

## Stage 5: Message Rendering

**10. Message bubble and indicators** - Render assistant output as markdown with syntax-highlighted code, show the model and approximate token chip, present the error/stopped states with a retry action, and surface the degraded-memory notice. Reference spec §3 for the markdown choice and §5 for status semantics.

**11. Recalled turns indicator** - Add the expandable "Recalled N earlier turns" panel under assistant messages that lists retrieved turns with timestamps and similarity, sourced from the persisted recalls returned by the messages endpoint per spec §5 and §6.
