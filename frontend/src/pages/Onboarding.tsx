import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api, type ProviderName, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { settingsKey } from "@/hooks/useSettings";

const PROVIDERS: { name: ProviderName; label: string; link: string }[] = [
  { name: "anthropic", label: "Anthropic", link: "https://console.anthropic.com/settings/keys" },
  { name: "openai", label: "OpenAI", link: "https://platform.openai.com/api-keys" },
  { name: "openai_compat", label: "Local (Ollama / LM Studio)", link: "https://ollama.com/" },
];

export function OnboardingPage() {
  const [provider, setProvider] = useState<ProviderName>("anthropic");
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const nav = useNavigate();
  const qc = useQueryClient();

  const onSave = async () => {
    setBusy(true);
    try {
      await api.putKey(provider, key);
      toast.success("Provider configured");
      await qc.invalidateQueries({ queryKey: settingsKey });
      nav("/settings");
    } catch (e) {
      const msg = e instanceof ApiError ? e.body.error.message : String(e);
      toast.error(msg);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mx-auto mt-24 max-w-xl">
      <Card>
        <CardHeader>
          <CardTitle>Welcome to agent-maker</CardTitle>
          <CardDescription>
            Add at least one provider API key to start building agents.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            {PROVIDERS.map((p) => (
              <Button
                key={p.name}
                variant={provider === p.name ? "default" : "outline"}
                size="sm"
                onClick={() => setProvider(p.name)}
              >
                {p.label}
              </Button>
            ))}
          </div>
          <div className="space-y-2">
            <Label htmlFor="key">API key</Label>
            <Input
              id="key"
              type="password"
              autoComplete="off"
              placeholder={provider === "openai_compat" ? "Optional for local endpoints" : "sk-..."}
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Get a key at{" "}
              <a className="underline" href={PROVIDERS.find((p) => p.name === provider)!.link} target="_blank" rel="noreferrer">
                {PROVIDERS.find((p) => p.name === provider)!.link}
              </a>
            </p>
          </div>
          <Button disabled={busy || (!key && provider !== "openai_compat")} onClick={onSave}>
            {busy ? "Saving…" : "Save and continue"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
