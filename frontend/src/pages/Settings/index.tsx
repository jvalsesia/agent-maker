import { useTranslation } from "react-i18next";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ProvidersSection } from "./Providers";
import { MemoryDefaultsSection } from "./MemoryDefaults";
import { AppearanceSection } from "./Appearance";
import { DataSection } from "./Data";

export function SettingsPage() {
  const { t } = useTranslation();
  return (
    <div className="mx-auto max-w-3xl">
      <h1 className="text-2xl font-semibold tracking-tight">{t("settings.title")}</h1>
      <p className="text-sm text-muted-foreground mt-1">{t("settings.subtitle")}</p>
      <Tabs defaultValue="providers" className="mt-6">
        <TabsList>
          <TabsTrigger value="providers">{t("settings.tabs.providers")}</TabsTrigger>
          <TabsTrigger value="memory">{t("settings.tabs.memory")}</TabsTrigger>
          <TabsTrigger value="appearance">{t("settings.tabs.appearance")}</TabsTrigger>
          <TabsTrigger value="data">{t("settings.tabs.data")}</TabsTrigger>
        </TabsList>
        <TabsContent value="providers"><ProvidersSection /></TabsContent>
        <TabsContent value="memory"><MemoryDefaultsSection /></TabsContent>
        <TabsContent value="appearance"><AppearanceSection /></TabsContent>
        <TabsContent value="data"><DataSection /></TabsContent>
      </Tabs>
    </div>
  );
}
