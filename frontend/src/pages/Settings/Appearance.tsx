import { toast } from "sonner";
import { ApiError } from "@/lib/api";
import { useSettings, useUpdateSettings } from "@/hooks/useSettings";
import { applyTheme, type Theme } from "@/hooks/useTheme";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

const THEMES: Theme[] = ["light", "dark", "system"];

export function AppearanceSection() {
  const { data } = useSettings();
  const update = useUpdateSettings();
  const current = data?.appearance.theme ?? "system";

  const setTheme = async (t: Theme) => {
    try {
      await update.mutateAsync({ appearance: { theme: t } });
      applyTheme(t);
      toast.success(`Theme: ${t}`);
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Appearance</CardTitle>
        <CardDescription>Light, dark, or follow the system setting.</CardDescription>
      </CardHeader>
      <CardContent className="flex gap-2">
        {THEMES.map((t) => (
          <Button
            key={t}
            variant={current === t ? "default" : "outline"}
            size="sm"
            onClick={() => setTheme(t)}
          >
            {t}
          </Button>
        ))}
      </CardContent>
    </Card>
  );
}
