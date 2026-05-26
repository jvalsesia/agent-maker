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
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Run as a non-root user and give it a writable home for the file-backed
# secret store and any local state.
RUN useradd --create-home --uid 10001 appuser \
    && mkdir -p /data \
    && chown -R appuser:appuser /data

WORKDIR /app
COPY --from=builder /app/target/release/agent-maker /usr/local/bin/agent-maker

USER appuser

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

CMD ["agent-maker"]
