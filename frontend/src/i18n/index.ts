import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import ptBR from "./locales/pt-BR.json";
import { DEFAULT_LOCALE, SUPPORTED_CODES, resolveBrowserLocale } from "./locales/supported";

// Seed the initial language from the browser before settings load, so the very
// first paint matches the user's language; the Gate then applies the persisted
// locale (the source of truth) once settings arrive.
const initialLocale =
  typeof navigator !== "undefined" ? resolveBrowserLocale(navigator.language) : DEFAULT_LOCALE;

/**
 * react-i18next bootstrap. The active locale is owned by the settings singleton
 * (persisted server-side) and applied in the router Gate, so we deliberately do
 * NOT add a browser language detector here — initial detection lives in
 * `resolveBrowserLocale` and is only used to seed a default. Missing keys fall
 * back to English, and `returnNull: false` guarantees a string is always
 * rendered (never a raw key swallowed as null).
 */
i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "pt-BR": { translation: ptBR },
  },
  lng: initialLocale,
  fallbackLng: DEFAULT_LOCALE,
  supportedLngs: SUPPORTED_CODES,
  interpolation: { escapeValue: false },
  returnNull: false,
  // A missing key shows the English value via fallbackLng; if that is also
  // missing, i18next returns the key itself — acceptable as a last resort.
  saveMissing: false,
});

export default i18n;
