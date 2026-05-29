# syntax=docker/dockerfile:1

# ---- Build stage ----
FROM node:22-bookworm-slim AS builder

# pnpm via corepack, pinned for reproducible installs (lockfileVersion 9.0).
RUN corepack enable && corepack prepare pnpm@11.1.2 --activate

WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
# esbuild (via Vite) ships a postinstall that links its native binary. pnpm
# refuses ignored build scripts (ERR_PNPM_IGNORED_BUILDS); esbuild is the only
# dependency with a build script in this tree, so allow builds to run.
RUN pnpm install --frozen-lockfile --config.dangerouslyAllowAllBuilds=true

COPY . .
# Clerk publishable key is inlined by Vite at build time (it ships to the
# browser by design, so baking it into the image is safe). Absent → the SPA
# builds with the login wall disabled. Switching keys requires a rebuild.
ARG VITE_CLERK_PUBLISHABLE_KEY
ENV VITE_CLERK_PUBLISHABLE_KEY=$VITE_CLERK_PUBLISHABLE_KEY
RUN pnpm build

# ---- Runtime stage ----
FROM nginx:1.27-alpine AS runtime

# envsubst (from gettext) renders the nginx template at container start so the
# backend upstream can be injected per environment (compose vs. Railway).
RUN apk add --no-cache gettext curl

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf.template /etc/nginx/templates/default.conf.template
COPY docker-entrypoint.frontend.sh /docker-entrypoint.frontend.sh
RUN chmod +x /docker-entrypoint.frontend.sh

# BACKEND_URL is the upstream the /api and /sse routes proxy to.
# PORT is the port nginx listens on (Railway injects it; defaults to 80).
ENV BACKEND_URL=http://backend:8787 \
    PORT=80

EXPOSE 80

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=5 \
    CMD curl -fsS "http://127.0.0.1:${PORT:-80}/healthz" || exit 1

ENTRYPOINT ["/docker-entrypoint.frontend.sh"]
