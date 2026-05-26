import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import i18n from "@/i18n";
import { DEFAULT_LOCALE, isSupported, type Locale } from "@/i18n/locales/supported";
import { useUpdateSettings } from "@/hooks/useSettings";

/**
 * Reads the active UI locale from the i18next runtime. This is intentionally
 * decoupled from react-query: the persisted setting is the source of truth and
 * is applied to i18next by the router Gate (via {@link applyLocale}), so a
 * plain read here needs no QueryClient and works in any rendered component.
 */
export function useLocale(): Locale {
  const { i18n } = useTranslation();
  return isSupported(i18n.language) ? i18n.language : DEFAULT_LOCALE;
}

/**
 * Returns a setter that persists the locale through the settings mutation and
 * switches the i18next runtime language in place (no reload). Lives apart from
 * {@link useLocale} so read-only consumers don't take a react-query dependency.
 */
export function useLocaleSetter() {
  const update = useUpdateSettings();
  const setLocale = useCallback(
    async (next: Locale) => {
      await update.mutateAsync({ appearance: { locale: next } });
      await i18n.changeLanguage(next);
      document.documentElement.lang = next;
    },
    [update],
  );
  return { setLocale, isPending: update.isPending };
}

/** Applies a locale to the i18next runtime and `<html lang>` without persisting. */
export function applyLocale(locale: Locale) {
  if (i18n.language !== locale) i18n.changeLanguage(locale);
  document.documentElement.lang = locale;
}
