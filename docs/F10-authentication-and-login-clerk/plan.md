# Implementation Plan: Authentication and Login (Clerk)

**Prerequisites:**
- Backend dependency: `jsonwebtoken = "9"` (add to `backend/Cargo.toml`). `reqwest`, `tower-http` (with `cors`), and `async-trait` are already present.
- Frontend dependency: `@clerk/clerk-react` (add via `pnpm add @clerk/clerk-react`). `react-router-dom` v6 and `@tanstack/react-query` already present.
- A Clerk application with the Email/Password strategy enabled (sign-in + sign-up), providing the publishable key and the instance's Frontend API domain.
- Environment variables:
  - Backend: `CLERK_JWKS_URL` (the instance `/.well-known/jwks.json`), `CLERK_ISSUER`, and optional `CORS_ALLOWED_ORIGIN`. Both Clerk vars must be set together in any environment where the wall should be enforced; setting only one fails fast at startup. With both unset, auth runs in disabled (pass-through) mode for local dev and tests.
  - Frontend: `VITE_CLERK_PUBLISHABLE_KEY`. Absent → auth disabled on the client.
- Update `.env.example` and `docs/deployment.md` with the new variables.

## Stage 1: Backend authentication core

**1. Configuration and errors** - Extend the config loader to read the Clerk and CORS variables, returning a clear startup error when exactly one Clerk variable is provided. Add the `Unauthorized` (401) and `AuthUnavailable` (503) error variants to the shared error type, following the existing error envelope and `IntoResponse` mapping. See spec Sections 3, 5, and the Component Overview.

**2. JWKS cache** - Create the `auth` module with an in-memory JWKS cache that fetches and parses Clerk's RSA keys into decoding keys, looks them up by key ID, and lazily re-fetches (with a single retry) when the cache is cold or a key ID is missing, to tolerate key rotation. See spec Section 3 and the `jwks` component.

**3. Claims and token validation** - Implement the `Claims` type and the token-validation logic that verifies the signature against the matching cached key and confirms expiry and issuer. Expose `Claims` as a request-parts extractor that reads validated claims from request extensions. See spec Section 5 (validated claims) and the `claims` component.

## Stage 2: Backend wiring and route guarding

**4. Auth state in app state** - Build the auth state (cache + config + enabled flag) during app construction and place it in the shared application state, alongside a test-only constructor that seeds an enabled cache with a known key for integration tests. See spec Sections 3 and 7.

**5. require_auth middleware** - Implement the `require_auth` middleware that extracts the bearer token, validates it, and either injects the claims or returns the appropriate auth error; when auth is disabled it passes through and logs a one-time warning. See spec Section 2 diagram and the `middleware` component.

**6. Router split and CORS** - Restructure the router so the health route is registered on a public sub-router while all other `/api` routes form a protected sub-router carrying the `require_auth` layer, then merge them under `/api`. Add the configurable CORS layer that permits the `Authorization` header and the configured origin. See spec Section 3 (public-route carve-out, CORS) and Component Overview.

## Stage 3: Frontend authentication shell

**7. Clerk provider and token bridge** - Mount the Clerk provider at the app root, conditional on the publishable key being present, and add the token-bridge component that registers Clerk's token getter and an unauthorized handler with the API layer. See spec Section 3 (provider gating, token retrieval) and the `App`/`ClerkTokenBridge` components.

**8. Bearer injection and 401 handling** - Update the central `request()` fetch wrapper to attach the bearer token when a getter is registered and to invoke the registered unauthorized handler on a 401 response. See spec Section 5 and the `api.ts` component.

**9. Sign-in / sign-up pages and route guard** - Add the sign-in and sign-up pages rendering Clerk's hosted components, and a `RequireAuth` guard that redirects signed-out users to sign-in and otherwise renders its children. Register the public auth routes and wrap the existing protected route group (above the current onboarding `Gate`) with the guard. See spec Section 2 diagram and the router/components.

**10. Navigation identity** - Add the Clerk account button and signed-in identity to the navigation sidebar when auth is enabled. See spec Component Overview (`NavSidebar`).

## Stage 4: Internationalization and finalization

**11. Auth translations** - Add the `auth` namespace strings (page titles, sign-in/sign-up toggle, session-expired notice) to both the English and Brazilian Portuguese catalogs in parity. See spec Component Overview (i18n) and PRD Experience.

**12. Environment and deployment docs** - Record the new backend and frontend environment variables in `.env.example` and the deployment guide, including the enforce-vs-disabled behavior and the production requirement to set both Clerk variables and the publishable key. See plan Prerequisites and spec Section 3 gating decision.
