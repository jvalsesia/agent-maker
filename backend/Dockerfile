# syntax=docker/dockerfile:1

# ---- Build stage ----
# Rust edition 2024 requires a recent toolchain (>= 1.85).
FROM rust:1-bookworm AS builder

WORKDIR /app

# Cache dependency compilation: copy manifests first, build a stub, then the
# real sources. Migrations are embedded into the binary via sqlx::migrate!,
# so they must be present at compile time.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && echo "" > src/lib.rs \
    && cargo build --release --locked 2>/dev/null || true \
    && rm -rf src

COPY src ./src
COPY migrations ./migrations

# Touch sources so the cached stub objects are rebuilt with real code.
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked --bin agent-maker

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime

# rustls is used for TLS, but CA certificates are still needed to validate
# outbound HTTPS calls (LLM providers, embeddings, etc.).
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl gosu \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root runtime user and give it a writable home for the file-backed
# secret store and any local state. On Railway the persistent volume mounts over
# /data owned by root, so the entrypoint re-chowns it at startup before dropping
# privileges (see docker-entrypoint.sh).
RUN useradd --create-home --uid 10001 appuser \
    && mkdir -p /data \
    && chown -R appuser:appuser /data

WORKDIR /app
COPY --from=builder /app/target/release/agent-maker /usr/local/bin/agent-maker
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# NOTE: intentionally NOT setting `USER appuser` here — the entrypoint starts as
# root to fix the mounted volume's ownership, then execs the app via `gosu appuser`.

# The OS keychain (Secret Service) is unavailable in containers, so force the
# encrypted file store. AGENT_MAKER_HOME points at a writable volume.
# PORT is honored by Config::from_env (binds 0.0.0.0:$PORT). Railway overrides
# PORT at deploy time; locally it defaults to 8787.
ENV AGENT_MAKER_FORCE_FILE_STORE=1 \
    AGENT_MAKER_HOME=/data \
    PORT=8787 \
    RUST_LOG=agent_maker=info,tower_http=info,sqlx=warn

EXPOSE 8787

HEALTHCHECK --interval=15s --timeout=3s --start-period=20s --retries=5 \
    CMD curl -fsS "http://127.0.0.1:${PORT:-8787}/api/health" || exit 1

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["agent-maker"]
