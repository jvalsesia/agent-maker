import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useAgentModels, useCreateAgent } from "@/hooks/useAgents";
import { useSettings } from "@/hooks/useSettings";
import { ApiError, type ProviderName } from "@/lib/api";

const PROVIDERS: ProviderName[] = ["anthropic", "openai", "openai_compat"];

interface Props {
  open: boolean;
  pending: boolean;
  onClose: () => void;
  /** Called with the freshly created child id + chosen alias to attach it. */
  onCreated: (childId: string, alias: string | undefined) => void;
}

/**
 * Compact create-and-attach flow (F11): creates a brand-new agent with the
 * minimum F02 fields, then hands its id back to the parent section to attach.
 */
export function NewSubagentDialog({ open, pending, onClose, onCreated }: Props) {
  const { t } = useTranslation();
  const { data: settings } = useSettings();
  const create = useCreateAgent();

  const [name, setName] = useState("");
  const [systemPrompt, setSystemPrompt] = useState("");
  const [alias, setAlias] = useState("");
  const [provider, setProvider] = useState<ProviderName>("anthropic");
  const [model, setModel] = useState("");

  const { data: models } = useAgentModels(provider);

  useEffect(() => {
    if (open) {
      setName("");
      setSystemPrompt("");
      setAlias("");
      const p = settings?.default_provider ?? "anthropic";
      setProvider(p);
      setModel(settings?.default_model?.[p] ?? "");
    }
  }, [open, settings]);

  // Default the model to the first available when none is chosen for a provider.
  useEffect(() => {
    if (!model && models?.models?.length) setModel(models.models[0]);
  }, [models, model]);

  const busy = pending || create.isPending;

  const onSubmit = async () => {
    if (!name.trim() || !systemPrompt.trim() || !model) return;
    try {
      const resp = await create.mutateAsync({
        name: name.trim(),
        system_prompt: systemPrompt.trim(),
        provider,
        model,
      });
      onCreated(resp.agent.id, alias.trim() || undefined);
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("agents.newSubagent.title")}</DialogTitle>
          <DialogDescription>{t("agents.newSubagent.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">
              {t("agents.newSubagent.nameLabel")}
            </label>
            <Input value={name} onChange={(e) => setName(e.target.value)} maxLength={60} />
          </div>
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">
              {t("agents.newSubagent.promptLabel")}
            </label>
            <Textarea
              rows={4}
              value={systemPrompt}
              onChange={(e) => setSystemPrompt(e.target.value)}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">
                {t("agents.newSubagent.providerLabel")}
              </label>
              <Select
                value={provider}
                onValueChange={(v) => {
                  setProvider(v as ProviderName);
                  setModel("");
                }}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {PROVIDERS.map((p) => (
                    <SelectItem key={p} value={p}>
                      {p}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1">
              <label className="text-xs font-medium text-muted-foreground">
                {t("agents.newSubagent.modelLabel")}
              </label>
              <Select value={model} onValueChange={setModel}>
                <SelectTrigger>
                  <SelectValue placeholder={t("agents.newSubagent.modelPlaceholder")} />
                </SelectTrigger>
                <SelectContent>
                  {(models?.models ?? []).map((m) => (
                    <SelectItem key={m} value={m}>
                      {m}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
          <div className="space-y-1">
            <label className="text-xs font-medium text-muted-foreground">
              {t("agents.newSubagent.aliasLabel")}
            </label>
            <Input
              value={alias}
              onChange={(e) => setAlias(e.target.value)}
              placeholder={t("agents.newSubagent.aliasPlaceholder")}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={onSubmit}
            disabled={busy || !name.trim() || !systemPrompt.trim() || !model}
          >
            {busy ? t("common.saving") : t("agents.newSubagent.create")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
