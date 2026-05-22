import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Eye, EyeOff } from "lucide-react";
import { ApiError, api, type ProviderEntry, type ProviderName } from "@/lib/api";
import { useSettings, settingsKey } from "@/hooks/useSettings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";

const LABELS: Record<ProviderName, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  openai_compat: "Local (OpenAI-compatible)",
};

export function ProvidersSection() {
  const { data } = useSettings();
  if (!data) return <p className="text-sm text-muted-foreground">Loading…</p>;
  return (
    <div className="space-y-4">
      {data.providers.map((p) => (
        <ProviderCard key={p.name} entry={p} />
      ))}
      {data.key_store_backend === "file" && (
        <p className="text-xs text-muted-foreground border border-border rounded-md p-3">
          Using encrypted file fallback — OS keychain unavailable.
        </p>
      )}
    </div>
  );
}

function ProviderCard({ entry }: { entry: ProviderEntry }) {
  const qc = useQueryClient();
  const [key, setKey] = useState("");
  const [baseUrl, setBaseUrl] = useState(entry.base_url ?? "");
  const [reveal, setReveal] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testMsg, setTestMsg] = useState<string | null>(null);

  const save = async () => {
    try {
      await api.putKey(entry.name, key || "", baseUrl || null);
      toast.success(`${LABELS[entry.name]} saved`);
      setKey("");
      await qc.invalidateQueries({ queryKey: settingsKey });
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const remove = async () => {
    try {
      await api.deleteKey(entry.name);
      toast.success(`${LABELS[entry.name]} removed`);
      await qc.invalidateQueries({ queryKey: settingsKey });
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const test = async () => {
    setTesting(true);
    setTestMsg(null);
    try {
      const r = await api.testProvider(entry.name);
      setTestMsg(`OK — ${r.model_used} (${r.latency_ms} ms)`);
    } catch (e) {
      setTestMsg(e instanceof ApiError ? e.body.error.message : String(e));
    } finally {
      setTesting(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center justify-between">
          <span>{LABELS[entry.name]}</span>
          <span className="text-xs font-normal text-muted-foreground">
            {entry.key_configured ? "configured" : "not configured"}
          </span>
        </CardTitle>
        {entry.key_configured && entry.key_masked && (
          <CardDescription className="font-mono text-xs">
            {reveal ? entry.key_masked : "•".repeat(entry.key_masked.length)}
            <button
              type="button"
              onClick={() => setReveal((r) => !r)}
              className="ml-2 align-middle text-muted-foreground hover:text-foreground"
              aria-label={reveal ? "hide key" : "show key"}
            >
              {reveal ? <EyeOff className="inline h-3 w-3" /> : <Eye className="inline h-3 w-3" />}
            </button>
          </CardDescription>
        )}
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="space-y-1">
          <Label htmlFor={`key-${entry.name}`}>API key</Label>
          <Input
            id={`key-${entry.name}`}
            type="password"
            autoComplete="off"
            placeholder={entry.key_configured ? "Replace key (leave empty to keep)" : "Paste key here"}
            value={key}
            onChange={(e) => setKey(e.target.value)}
          />
        </div>
        {entry.name === "openai_compat" && (
          <div className="space-y-1">
            <Label htmlFor="base">Base URL</Label>
            <Input
              id="base"
              placeholder="http://localhost:11434/v1"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
            />
          </div>
        )}
        <div className="flex flex-wrap gap-2">
          <Button onClick={save} disabled={!key && entry.name !== "openai_compat"}>
            Save
          </Button>
          <Button variant="outline" onClick={test} disabled={!entry.key_configured && entry.name !== "openai_compat"}>
            {testing ? "Testing…" : "Test connection"}
          </Button>
          {entry.key_configured && (
            <Button variant="destructive" onClick={remove}>
              Remove key
            </Button>
          )}
        </div>
        {testMsg && (
          <p className={`text-xs ${testMsg.startsWith("OK") ? "text-green-600" : "text-destructive"}`}>{testMsg}</p>
        )}
      </CardContent>
    </Card>
  );
}
