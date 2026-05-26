/**
 * Locale-aware formatting helpers built on the native `Intl` API. Centralizing
 * them keeps date/number presentation consistent and switchable by locale
 * without pulling in a formatting library. Callers pass the active locale
 * (from `useLocale`); a bad/empty locale falls back to the runtime default.
 */
import { DEFAULT_LOCALE, type Locale } from "@/i18n/locales/supported";

function toDate(value: Date | string | number): Date {
  return value instanceof Date ? value : new Date(value);
}

/** Short calendar date, e.g. `en` → `May 26, 2026`, `pt-BR` → `26 de mai. de 2026`. */
export function formatDate(value: Date | string | number, locale: Locale = DEFAULT_LOCALE): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(toDate(value));
}

/** Date with time, e.g. `May 26, 2026, 9:41 AM`. */
export function formatDateTime(
  value: Date | string | number,
  locale: Locale = DEFAULT_LOCALE,
): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(toDate(value));
}

/** Plain integer/decimal with locale grouping, e.g. `1,234` vs `1.234`. */
export function formatNumber(value: number, locale: Locale = DEFAULT_LOCALE): string {
  return new Intl.NumberFormat(locale).format(value);
}

/**
 * Percentage from a 0..1 fraction, e.g. `0.42` → `42%`. `maximumFractionDigits`
 * defaults to 0 to match the existing `(x * 100).toFixed(0)` call sites.
 */
export function formatPercent(
  fraction: number,
  locale: Locale = DEFAULT_LOCALE,
  maximumFractionDigits = 0,
): string {
  return new Intl.NumberFormat(locale, {
    style: "percent",
    maximumFractionDigits,
  }).format(fraction);
}
