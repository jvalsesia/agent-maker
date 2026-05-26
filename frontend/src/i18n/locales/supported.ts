/**
 * Supported UI locales for F09. Mirrors the backend `SUPPORTED_LOCALES`
 * (`backend/src/i18n.rs`). Adding a locale means a new entry here, a matching
 * message catalog, and widening the backend CHECK constraints.
 */

export type Locale = "en" | "pt-BR";

export const DEFAULT_LOCALE: Locale = "en";

export interface LocaleInfo {
  /** BCP 47 code, used in the DB/API and as the i18next language key. */
  code: Locale;
  /** Name shown in the language selector, written in that language. */
  nativeName: string;
}

export const SUPPORTED_LOCALES: LocaleInfo[] = [
  { code: "en", nativeName: "English" },
  { code: "pt-BR", nativeName: "Português (Brasil)" },
];

export const SUPPORTED_CODES: Locale[] = SUPPORTED_LOCALES.map((l) => l.code);

export function isSupported(code: string): code is Locale {
  return (SUPPORTED_CODES as string[]).includes(code);
}

/**
 * Maps a browser language tag (`navigator.language`) to the nearest supported
 * locale, falling back to {@link DEFAULT_LOCALE}. An exact match wins; a bare
 * primary subtag match comes next (e.g. `pt`, `pt-PT` → `pt-BR`); anything
 * unrecognized resolves to English.
 */
export function resolveBrowserLocale(tag: string | undefined | null): Locale {
  if (!tag) return DEFAULT_LOCALE;
  if (isSupported(tag)) return tag;
  const primary = tag.split("-")[0]?.toLowerCase();
  const byPrimary = SUPPORTED_CODES.find(
    (code) => code.split("-")[0]?.toLowerCase() === primary,
  );
  return byPrimary ?? DEFAULT_LOCALE;
}
