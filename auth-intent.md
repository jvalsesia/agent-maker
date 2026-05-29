# Project Intent: Clerk Authentication Integration for agent-maker
## Executive Summary
The goal of this initiative is to integrate Clerk as the primary Authentication and Identity Provider into an existing full-stack monorepo agent-maker application. The architecture enforces a decoupling of responsibilities: the ReactJS frontend handles user interfaces, session state, and initial token acquisition, while the Rust Axum backend securely validates incoming operations statelessly using standard cryptography.

## Core Architecture & Token Lifecycle
To ensure maximum framework stability, minimal build bloat, and independence from third-party crate release cycles, the backend will omit vendor-specific SDKs (clerk-rs) and rely entirely on Pure JWT Verification.

```mermaid
sequenceDiagram
    autonumber
    actor User as User Browser
    participant React as React Frontend
    participant Clerk as Clerk Auth Servers
    participant Axum as Axum Backend

    %% Phase 1: Authentication
    Note over User, Clerk: Phase 1: User Authentication
    User->>React: Enters Email/Password
    React->>Clerk: Submit credentials via Clerk SDK
    Clerk-->>React: Auth Success (Sets Session Cookie)
    React-->>User: Render authenticated UI state

    %% Phase 2: In-Memory Key Caching
    Note over Axum, Clerk: Phase 2: Startup / Lazy Key Caching
    Axum->>Clerk: HTTP GET /.well-known/jwks.json
    Clerk-->>Axum: Returns Public Key Set (JWKS)
    Note over Axum: Caches Public Keys in memory<br/>(Avoids network call per request)

    %% Phase 3: Protected API Request
    Note over User, Axum: Phase 3: Authenticated API Lifecycle
    User->>React: Triggers action requiring protected data
    React->>React: Await clerk.getToken()
    Note over React: Retrieves short-lived JWT<br/>from memory/cookie
    React->>Axum: HTTP GET /api/protected-resource<br/>Authorization: Bearer <JWT>
    
    %% Phase 4: Token Verification
    Note over Axum: Custom Axum Extractor:<br/>1. Extracts Bearer string<br/>2. Finds matching key by 'kid'<br/>3. Validates signature, 'exp', and 'iss'
    
    alt JWT is Valid
        Axum->>Axum: Inject Claims into Route Handler
        Axum-->>React: HTTP 200 OK (Protected Data Payload)
        React-->>User: Render data to viewport
    else JWT is Invalid / Expired
        Axum-->>React: HTTP 401 Unauthorized
        React->>React: Handle session expiry / redirect to login
    end
```

## Technology Stack & Key Dependencies
- **Identity Provider:** Clerk (Email/Password authentication strategy).

- **Frontend Client:** ReactJS using @clerk/clerk-react.

- **Backend Server:** Rust Axum using standard ecosystem crates:

    - jsonwebtoken: For decoding tokens and cryptographic validation.

    - reqwest: For pulling Clerk's JSON Web Key Set (JWKS).

    - serde / serde_json: For deserializing token payloads.

    - tower-http: For robust CORS management (allowing Authorization headers).

## Operational Strategy (Phase-by-Phase)
### Phase 1: Configuration & Provisioning
- **Clerk Console:** Establish a new application environment, explicitly enabling the Email/Password authentication provider.

- **Environment Secret Distribution:**

    - **Frontend:** VITE_CLERK_PUBLISHABLE_KEY (or equivalent bundler prefix).
    - **Backend:** CLERK_JWKS_URL (pointing to your application's /.well-known/jwks.json endpoint) and CLERK_ISSUER string.

### Phase 2: React Frontend Blueprint
- **Context Layering:** Inject <ClerkProvider> at the root node of the client application.

- **UI Placement:** Render standard Clerk components (<SignIn/>, <SignUp/>, <UserButton/>) inside explicit, routed components.

- API Interceptor Pattern: Configure the global HTTP/Fetch client wrapper to asynchronously fetch the current session token utilizing Clerk’s useAuth().getToken() routine before injecting it directly into the Authorization: Bearer <TOKEN> header of every outgoing backend request.

### Phase 3: Rust Axum Backend Blueprint (Pure JWT)
- **Startup Cache Hook:** Construct an asynchronous startup utility that calls the CLERK_JWKS_URL to fetch Clerk's active public keys, deserializing them into a structured format to keep resident in application memory (axum::Extension or state).

- **Custom Axum Extractor:**

    1. Implement a custom struct struct Claims implementing Axum's FromRequestParts trait.

    2. Extract the string payload from the HTTP Authorization header.

    3. Look up the appropriate cryptographic key from the locally cached JWKS based on the incoming token header's kid (Key ID).

    4. Decode and evaluate the token signature, confirming expiry (exp) is in the future, and the issuer (iss) matches your project's unique Clerk instance.

- **Route Guarding:** Inject the Claims extractor into any target route handler requiring authentication.

## Security & Edge Case Guardrails
- **JWKS Key Rotation:** While the initial target will fetch keys at startup, the system design must eventually allow lazy re-fetching if a signature verification fails, accommodating Clerk's background key rotation schedules without server restarts.

- **CORS Hardening:** Axum’s tower-http::cors::CorsLayer must be strictly configured to allow the Authorization header and your specific frontend origin.

- **Database Alignment (Future Extension):** Should the backend require a localized mapping of user data (e.g., profiles, relational data), a dedicated, unauthenticated endpoint will be mapped out later to ingest secure, signature-verified Clerk Webhooks corresponding to user.created events.