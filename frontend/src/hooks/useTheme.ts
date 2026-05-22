import { useEffect } from "react";

export type Theme = "light" | "dark" | "system";

export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  const isDark =
    theme === "dark" ||
    (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  root.classList.toggle("dark", isDark);
}

export function useApplyTheme(theme: Theme | undefined) {
  useEffect(() => {
    if (theme) applyTheme(theme);
  }, [theme]);
}
