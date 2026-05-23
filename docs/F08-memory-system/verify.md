# F08 — Manual verification

These are the end-to-end smoke checks listed in spec §9. They assume
`docker compose up -d` has been run and `cargo run` + `pnpm dev` are
serving the app.

## Setup

1. Open Settings and paste a real OpenAI key. Click "Test connection"
   for OpenAI and confirm a green check.
2. Pick (or create) an agent and open it. Note its `id`.
3. Insert ≥ 20 user/assistant messages directly into the agent's first
   conversation via psql:

   ```sql
   INSERT INTO messages (conversation_id, role, content)
   SELECT '<conv-uuid>', CASE WHEN g % 2 = 0 THEN 'user' ELSE 'assistant' END,
          'message-' || g
   FROM generate_series(1, 20) g;
   ```

## Embed-pending is idempotent

```bash
curl -X POST localhost:3000/api/conversations/<conv-uuid>/memory/embed-pending
# → { "embedded": 20, "skipped_already_present": 0, "failed": 0 }

curl -X POST localhost:3000/api/conversations/<conv-uuid>/memory/embed-pending
# → { "embedded": 0,  "skipped_already_present": 20, "failed": 0 }
```

## Query returns top-K outside the recent-N window

```bash
curl -X POST localhost:3000/api/conversations/<conv-uuid>/memory/query \
     -H 'content-type: application/json' \
     -d '{ "query": "what did we say earlier?", "recent_n": 4, "top_k": 3 }'
```

Assert: `retrieved.length === 3`, each `id` is not in the last four
created messages, similarity scores are between 0 and 1.

## Clear via the UI

1. Open the chat surface for the agent.
2. Hover the conversation row in the sidebar; click the eraser icon.
3. Dialog shows "Embedded turns: 20".
4. Confirm "Clear memory". Toast: "Cleared 20 embedded turns".
5. Re-run `embed-pending` — `embedded` should equal 20 again.

## Degraded query when no OpenAI key

1. In Settings, remove the OpenAI key (Delete).
2. Repeat the `POST /memory/query` call.
3. Response: `"degraded": true, "degraded_reason": "no_openai_key",
   "retrieved": []`. The `recent` window is still populated.

## Per-agent overrides

1. In AgentForm, set Recent-N to 6 and Top-K to 2; save.
2. `POST /memory/query` with no `recent_n`/`top_k` in the body.
3. Response: `effective_n === 6`, `effective_k === 2`.
