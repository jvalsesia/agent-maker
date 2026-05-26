# Implementation Plan: Internationalization (i18n)

**Prerequisites:**
- Frontend deps: `i18next`, `react-i18next`, `i18next-browser-languagedetector` (added to `frontend/package.json`).
- Backend: existing Rust/Axum/sqlx stack; new migration applied via the standard `sqlx::migrate` flow.
- Dependencies F01, F02, F05, F07 implemented (all merged to main).
- Supported v1 locales: `en`, `pt-BR`.

### Stage 1: Persistence and Shared Locale Primitives

**1. Migration** - Add the `locale` column to the `settings` singleton and the `response_language` column to `agents`, each with a CHECK constraint bounding it to the supported set, following the existing `theme` CHECK pattern. See spec Section 6.

**2. Backend locale module** - Introduce a shared `i18n` module exposing the supported-locale set, a validation helper, and a locale-to-language-name lookup, to be consumed by settings, agents, and chat. See spec Section 4.

**3. Settings locale field** - Extend the settings DTOs and update path to read, validate, and persist `appearance.locale`, rejecting unsupported values with the standard validation error. See spec Sections 5 and 6.

### Stage 2: Agent Response Language and Chat Directive

**4. Agent response_language** - Add the `response_language` field to the agent model, upsert payload, and response, validating it as `auto` or a supported locale and persisting it through create, update, and clone. See spec Sections 4 and 6.

**5. Chat locale resolution** - Add the optional `locale` field to the chat start request and resolve the effective response language in ChatService using the agent override, then the request locale, then English. See spec Sections 3 and 5.

**6. Prompt directive injection** - Prepend a "respond in <language>" directive to the composed base system prompt before calling the locale-agnostic composer, adding nothing when the resolved language is English. See spec Section 2.

### Stage 3: Template Localization

**7. Localized catalog** - Add Brazilian Portuguese variant data to the static template catalog keyed by slug, with a lookup helper that returns the requested locale's variant or falls back to English. See spec Section 3.

**8. Template locale serving** - Accept a `locale` query parameter on the templates listing, return the matched variant, and flag responses served from the English fallback. See spec Section 5.

### Stage 4: Frontend i18n Runtime and UI

**9. i18n runtime and catalogs** - Bootstrap react-i18next with English and Portuguese message catalogs, the supported-locale registry with native display names and browser-language mapping, and an English fallback for missing keys. See spec Section 4.

**10. Locale application and hook** - Initialize i18n at app startup and apply the active locale in the Gate component alongside theme, exposing a hook that persists locale changes through the settings mutation and switches the runtime language without reload. See spec Sections 2 and 4.

**11. Locale-aware formatting** - Add centralized date and number formatting helpers built on native Intl and route the existing scattered formatting call sites through them. See spec Sections 3 and 4.

**12. Settings and agent UI** - Add the Language selector to the Appearance settings card and the Response Language dropdown (defaulting to Automatic) to the agent form, wiring both to the extended API. See spec Section 5.

**13. UI string extraction** - Replace hardcoded UI strings across navigation, buttons, forms, toasts, modals, and empty states with translation keys, and surface the template fallback indicator in the gallery. See spec Sections 1 and 4.
