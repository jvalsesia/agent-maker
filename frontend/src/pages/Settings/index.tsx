import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ProvidersSection } from "./Providers";
import { MemoryDefaultsSection } from "./MemoryDefaults";
import { AppearanceSection } from "./Appearance";
import { DataSection } from "./Data";

export function SettingsPage() {
  return (
    <div className="mx-auto max-w-3xl">
      <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
      <p className="text-sm text-muted-foreground mt-1">
        Configure providers, memory defaults, appearance, and local data.
      </p>
      <Tabs defaultValue="providers" className="mt-6">
        <TabsList>
          <TabsTrigger value="providers">Providers</TabsTrigger>
          <TabsTrigger value="memory">Memory</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
          <TabsTrigger value="data">Data</TabsTrigger>
        </TabsList>
        <TabsContent value="providers"><ProvidersSection /></TabsContent>
        <TabsContent value="memory"><MemoryDefaultsSection /></TabsContent>
        <TabsContent value="appearance"><AppearanceSection /></TabsContent>
        <TabsContent value="data"><DataSection /></TabsContent>
      </Tabs>
    </div>
  );
}
