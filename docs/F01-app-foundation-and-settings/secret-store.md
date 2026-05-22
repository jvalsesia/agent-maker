# Secret Store — Keyring vs File Fallback

agent-maker stores provider API keys in one of two backends. Both are documented
in the F01 spec; this note explains *which one to use, when, and why*.

## How the backend is chosen

On startup, `secrets::auto()` runs the following decision:

```
if AGENT_MAKER_FORCE_FILE_STORE=1 → file store
else if OS keychain reachable     → keyring store
else (probe fails)                → file store + WARN log + UI banner
```

The current backend is always reported at `GET /api/settings` → `key_store_backend`
(`"keyring"` or `"file"`).

## The two backends

### Keyring (`keyring` crate)

| Platform | Underlying service |
|---|---|
| macOS | Keychain |
| Windows | Credential Manager |
| Linux | Secret Service via D-Bus (gnome-keyring, KWallet, etc.) |

The OS protects the secret with the user's login session. The application never
sees the plaintext key after writing it — every read goes through an OS call
that can require an unlock prompt.

**Pros:** kernel/session-mediated protection; integrates with biometric or
PAM-based unlock; nothing decryptable-at-rest sits in the project home dir.

**Cons (Linux-specific):** Secret Service implementations are notoriously
inconsistent.

- A locked or session-only collection can return `NoEntry` for keys you just
  wrote — the metadata in Postgres says `key_configured: true` but
  `POST /api/settings/providers/:name/test` returns `no_key`.
- Headless servers, containers, WSL, and some minimal desktops have no Secret
  Service at all; the probe fails and the backend silently falls back to the
  file store — that is intentional and safe.

### File store (AES-256-GCM)

A self-contained encrypted vault stored under `AGENT_MAKER_HOME`
(default `~/.agent-maker`):

| File | Contents | Permissions |
|---|---|---|
| `master.key` | 16-byte salt + 32-byte random secret | `0600` |
| `secrets.bin` | 12-byte nonce + AES-256-GCM ciphertext of a JSON map | `0600` |

The 32-byte AES key is derived from the random secret via Argon2id at startup.
Each `put` re-encrypts the whole vault with a fresh nonce.

**Pros:** deterministic, fully offline, no D-Bus / desktop dependency, survives
logout and reboots, identical behavior across platforms.

**Cons:** an attacker who can read *both* `master.key` and `secrets.bin` can
decrypt your provider keys. `0600` mitigates this on a single-user box but
matters for shared filesystems or weak host security.

## When to use which

| Situation | Recommendation |
|---|---|
| macOS / Windows desktop | Keyring (default) |
| Linux desktop with reliable gnome-keyring / KWallet, unlocked at login | Keyring (default) |
| Linux desktop where saves appear to "vanish" between requests | File store — set `AGENT_MAKER_FORCE_FILE_STORE=1` |
| Headless / WSL / CI / container | File store (auto-selected; the probe fails) |
| Shared / multi-user host with weak file isolation | Keyring, *or* mount `AGENT_MAKER_HOME` on an encrypted volume |

## Switching backends

The two backends are independent — the metadata row in Postgres
(`provider_keys`) carries the mask + base_url; the actual secret lives only in
whichever backend was active at the time of writing. If you switch backends,
**re-save your keys** so the new backend has the data:

```bash
# 1. Wipe metadata + clear whatever the current store has
curl -s -X POST http://127.0.0.1:8787/api/settings/data/wipe \
  -H 'content-type: application/json' -d '{"confirm":"WIPE"}'

# 2. Restart with the desired backend (e.g. force file store)
AGENT_MAKER_FORCE_FILE_STORE=1 cargo run

# 3. Re-save keys via the UI or curl
```

If you don't wipe first, you can end up in a "configured but unreachable"
state: `GET /api/settings` shows the masked key, but `test` returns `no_key`
because the *active* store doesn't have it.

## Diagnostics

The keyring backend logs every operation:

- `keyring put ok` — secret written
- `keyring put failed` — backend rejected the write (look at the `error` field)
- `keyring get: entry not found` — read came back empty
- `keyring get failed` — backend errored

If `put ok` is logged but a subsequent `get` returns `entry not found`, your
Linux Secret Service is dropping the entry between calls — switch to the file
store.

## Reference

- Spec: [`spec.md`](./spec.md) section 2 ("Encrypted key storage") and section 6
  ("Error Handling — OS keychain unavailable")
- Code: `backend/src/secrets/{mod,keyring_store,file_store}.rs`
- Env var: `AGENT_MAKER_FORCE_FILE_STORE=1` in `.env`
