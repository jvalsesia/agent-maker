import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { ApiError } from "@/lib/api";
import { useSettings, useUpdateSettings } from "@/hooks/useSettings";
import { useLocale, useLocaleSetter } from "@/hooks/useLocale";
import { applyTheme, type Theme } from "@/hooks/useTheme";
import { SUPPORTED_LOCALES, type Locale } from "@/i18n/locales/supported";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

const THEMES: Theme[] = ["light", "dark", "system"];

export function AppearanceSection() {
  const { t } = useTranslation();
  const { data } = useSettings();
  const update = useUpdateSettings();
  const locale = useLocale();
  const { setLocale } = useLocaleSetter();
  const current = data?.appearance.theme ?? "system";

  const setTheme = async (theme: Theme) => {
    try {
      await update.mutateAsync({ appearance: { theme } });
      applyTheme(theme);
      toast.success(t("settings.appearance.themeChanged", { theme }));
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const onLocaleChange = async (next: Locale) => {
    try {
      await setLocale(next);
      const name = SUPPORTED_LOCALES.find((l) => l.code === next)?.nativeName ?? next;
      toast.success(t("settings.appearance.languageChanged", { language: name }));
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("settings.appearance.title")}</CardTitle>
        <CardDescription>{t("settings.appearance.description")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="flex gap-2">
          {THEMES.map((theme) => (
            <Button
              key={theme}
              variant={current === theme ? "default" : "outline"}
              size="sm"
              onClick={() => setTheme(theme)}
            >
              {theme}
            </Button>
          ))}
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="language-select">{t("settings.appearance.language")}</Label>
          <Select value={locale} onValueChange={(v) => onLocaleChange(v as Locale)}>
            <SelectTrigger id="language-select" className="max-w-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SUPPORTED_LOCALES.map((l) => (
                <SelectItem key={l.code} value={l.code}>
                  {l.nativeName}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            {t("settings.appearance.languageDescription")}
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
