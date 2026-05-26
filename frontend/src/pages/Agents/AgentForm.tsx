import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AlertTriangle, ChevronDown, ChevronRight, Copy, Trash2 } from "lucide-react";
import {
  ApiError,
  type AgentUpsert,
  type AgentWarning,
  type ProviderName,
  type ResponseLanguage,
} from "@/lib/api";
import { SUPPORTED_LOCALES } from "@/i18n/locales/supported";
import {
  useAgent,
  useAgentModels,
  useCloneAgent,
  useCreateAgent,
  useDeleteAgent,
  useDeleteAgentKey,
  useSaveAgentKey,
  useUpdateAgent,
} from "@/hooks/useAgents";
import { useSettings } from "@/hooks/useSettings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DeleteAgentDialog } from "./DeleteAgentDialog";
import { AgentSkillsPanel } from "./AgentSkillsPanel";

const PROVIDER_LABELS: Record<ProviderName, string> = {
  anthropic: "Anthropic",
  openai: "OpenAI",
  openai_compat: "Local (OpenAI-compatible)",
};

interface FieldErrors {
  [field: string]: string;
}

export function AgentForm() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const editing = !!id;
  const navigate = useNavigate();

  const existing = useAgent(id);
  const { data: settings } = useSettings();

  const [name, setName] = useState("");
  const [preamble, setPreamble] = useState("");
  const [systemPrompt, setSystemPrompt] = useState("");
  const [provider, setProvider] = useState<ProviderName>("anthropic");
  const [model, setModel] = useState("");
  const [recentN, setRecentN] = useState<string>("");
  const [topK, setTopK] = useState<string>("");
  const [responseLanguage, setResponseLanguage] = useState<ResponseLanguage>("auto");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [keyInput, setKeyInput] = useState("");
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [warnings, setWarnings] = useState<AgentWarning[]>([]);

  // Hydrate form when editing
  useEffect(() => {
    const a = existing.data?.agent;
    if (!a) return;
    setName(a.name);
    setPreamble(a.preamble ?? "");
    setSystemPrompt(a.system_prompt);
    setProvider(a.provider);
    setModel(a.model);
    setRecentN(a.recent_n_override?.toString() ?? "");
    setTopK(a.top_k_override?.toString() ?? "");
    setResponseLanguage(a.response_language ?? "auto");
  }, [existing.data?.agent]);

  const modelsQuery = useAgentModels(provider);

  const create = useCreateAgent();
  const update = useUpdateAgent(id ?? "");
  const clone = useCloneAgent();
  const del = useDeleteAgent();
  const saveKey = useSaveAgentKey(id ?? "");
  const deleteKey = useDeleteAgentKey(id ?? "");

  const [pendingDelete, setPendingDelete] = useState(false);

  const providerKeyConfigured = useMemo(() => {
    if (!settings) return true;
    return settings.providers.find((p) => p.name === provider)?.key_configured ?? false;
  }, [settings, provider]);

  const hasOverrideKey = existing.data?.agent?.has_override_key ?? false;
  const noKeyWarning = !providerKeyConfigured && !hasOverrideKey;

  const buildBody = (): AgentUpsert => ({
    name: name.trim(),
    preamble: preamble.trim() ? preamble.trim() : null,
    system_prompt: systemPrompt,
    provider,
    model,
    recent_n_override: recentN ? Number(recentN) : null,
    top_k_override: topK ? Number(topK) : null,
    response_language: responseLanguage,
  });

  const handleApiError = (e: unknown) => {
    if (e instanceof ApiError && e.body.error.field) {
      setFieldErrors({ [e.body.error.field]: e.body.error.message });
    } else {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const save = async (thenChat = false) => {
    setFieldErrors({});
    setWarnings([]);
    try {
      const body = buildBody();
      const resp = editing
        ? await update.mutateAsync(body)
        : await create.mutateAsync(body);
      setWarnings(resp.warnings);
      toast.success(t("agents.form.saved"));
      if (thenChat) {
        // F06/F07 will turn this into a fresh conversation. For now jump to the detail page.
        navigate(`/agents/${resp.agent.id}`);
      } else if (!editing) {
        navigate(`/agents/${resp.agent.id}`, { replace: true });
      }
    } catch (e) {
      handleApiError(e);
    }
  };

  const onClone = async () => {
    if (!id) return;
    try {
      const r = await clone.mutateAsync({ id });
      toast.success(t("agents.form.cloned", { name: r.agent.name }));
      navigate(`/agents/${r.agent.id}`);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onDelete = async () => {
    if (!id) return;
    try {
      await del.mutateAsync(id);
      toast.success(t("agents.form.deleted"));
      navigate("/agents");
    } catch (e) {
      handleApiError(e);
    }
  };

  const onSaveKey = async () => {
    if (!id || !keyInput.trim()) return;
    try {
      const r = await saveKey.mutateAsync(keyInput.trim());
      toast.success(t("agents.form.keySaved", { masked: r.key_masked }));
      setKeyInput("");
    } catch (e) {
      handleApiError(e);
    }
  };

  const onRemoveKey = async () => {
    if (!id) return;
    try {
      await deleteKey.mutateAsync();
      toast.success(t("agents.form.keyRemoved"));
    } catch (e) {
      handleApiError(e);
    }
  };

  const sysWarn = warnings.find((w) => w.field === "system_prompt");
  const saving = create.isPending || update.isPending;
  const agent = existing.data?.agent;

  return (
    <div className="mx-auto max-w-3xl pb-24">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            {editing ? agent?.name ?? t("agents.form.fallbackTitle") : t("agents.form.newTitle")}
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            <Link to="/agents" className="underline">
              {t("agents.form.allAgents")}
            </Link>
          </p>
        </div>
        {editing && id && (
          <Button variant="outline" onClick={() => navigate(`/agents/${id}/chat`)}>
            {t("agents.form.openChat")}
          </Button>
        )}
      </div>

      <div className="mt-6 space-y-5">
        <Field label={t("agents.form.name")} error={fieldErrors.name}>
          <Input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("agents.form.namePlaceholder")}
            maxLength={60}
          />
        </Field>

        <Field
          label={t("agents.form.preamble")}
          hint={`${preamble.length} / 500`}
          error={fieldErrors.preamble}
        >
          <Input
            value={preamble}
            onChange={(e) => setPreamble(e.target.value)}
            placeholder={t("agents.form.preamblePlaceholder")}
            maxLength={500}
          />
        </Field>

        <Field
          label={t("agents.form.systemPrompt")}
          hint={t("agents.form.systemPromptChars", { count: systemPrompt.length })}
          error={fieldErrors.system_prompt}
        >
          <Textarea
            value={systemPrompt}
            onChange={(e) => setSystemPrompt(e.target.value)}
            placeholder={t("agents.form.systemPromptPlaceholder")}
            rows={12}
            className="font-mono text-xs"
          />
          {sysWarn && (
            <p className="mt-1 flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
              <AlertTriangle className="h-3 w-3" /> {sysWarn.message}
            </p>
          )}
        </Field>

        <div className="grid grid-cols-2 gap-4">
          <Field label={t("agents.form.provider")} error={fieldErrors.provider}>
            <Select value={provider} onValueChange={(v) => { setProvider(v as ProviderName); setModel(""); }}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(["anthropic", "openai", "openai_compat"] as ProviderName[]).map((p) => (
                  <SelectItem key={p} value={p}>{PROVIDER_LABELS[p]}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            {noKeyWarning && (
              <p className="mt-1 flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
                <AlertTriangle className="h-3 w-3" /> {t("agents.form.noKeyWarning", { provider: PROVIDER_LABELS[provider] })}{" "}
                <Link to="/settings" className="underline">{t("agents.form.goToSettings")}</Link>
              </p>
            )}
          </Field>

          <Field label={t("agents.form.model")} error={fieldErrors.model}>
            <Select value={model} onValueChange={setModel}>
              <SelectTrigger>
                <SelectValue placeholder={modelsQuery.data ? t("agents.form.chooseModel") : t("common.loading")} />
              </SelectTrigger>
              <SelectContent>
                {(modelsQuery.data?.models ?? []).map((m) => (
                  <SelectItem key={m} value={m}>{m}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <Field
            label={t("agents.form.recentNOverride")}
            hint={t("agents.form.globalHint", { value: settings?.memory_defaults.recent_n ?? 10 })}
            error={fieldErrors.recent_n_override}
          >
            <Input
              type="number"
              min={4}
              max={30}
              value={recentN}
              onChange={(e) => setRecentN(e.target.value)}
              placeholder={t("agents.form.optional")}
            />
          </Field>
          <Field
            label={t("agents.form.topKOverride")}
            hint={t("agents.form.globalHint", { value: settings?.memory_defaults.top_k ?? 5 })}
            error={fieldErrors.top_k_override}
          >
            <Input
              type="number"
              min={0}
              max={10}
              value={topK}
              onChange={(e) => setTopK(e.target.value)}
              placeholder={t("agents.form.optional")}
            />
          </Field>
        </div>

        <Field
          label={t("agents.form.responseLanguage")}
          hint={t("agents.form.responseLanguageHint")}
        >
          <Select
            value={responseLanguage}
            onValueChange={(v) => setResponseLanguage(v as ResponseLanguage)}
          >
            <SelectTrigger className="max-w-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">{t("agents.form.responseLanguageAuto")}</SelectItem>
              {SUPPORTED_LOCALES.map((l) => (
                <SelectItem key={l.code} value={l.code}>
                  {l.nativeName}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Field>

        {editing && id && <AgentSkillsPanel agentId={id} />}

        {editing && (
          <div className="rounded-md border border-border">
            <button
              type="button"
              onClick={() => setShowAdvanced((v) => !v)}
              className="flex w-full items-center gap-2 px-4 py-3 text-sm font-medium"
            >
              {showAdvanced ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
              {t("agents.form.advanced")}
            </button>
            {showAdvanced && (
              <div className="border-t border-border px-4 py-4 space-y-3">
                {hasOverrideKey && (
                  <p className="text-xs text-muted-foreground">
                    {t("agents.form.overrideKeySet")}
                  </p>
                )}
                <div className="space-y-1">
                  <Label htmlFor="agent-key">{t("common.apiKey")}</Label>
                  <Input
                    id="agent-key"
                    type="password"
                    autoComplete="off"
                    placeholder={hasOverrideKey ? t("agents.form.replaceKeyPlaceholder") : t("agents.form.pasteKeyPlaceholder")}
                    value={keyInput}
                    onChange={(e) => setKeyInput(e.target.value)}
                  />
                </div>
                <div className="flex gap-2">
                  <Button onClick={onSaveKey} disabled={!keyInput.trim() || saveKey.isPending}>
                    {t("agents.form.saveKey")}
                  </Button>
                  {hasOverrideKey && (
                    <Button variant="outline" onClick={onRemoveKey} disabled={deleteKey.isPending}>
                      {t("agents.form.removeOverride")}
                    </Button>
                  )}
                </div>
              </div>
            )}
          </div>
        )}
      </div>

      <div className="mt-8 flex items-center justify-between border-t border-border pt-4">
        <Button
          variant="ghost"
          onClick={() => setPendingDelete(true)}
          className={editing ? "text-destructive" : "invisible"}
          disabled={!editing}
        >
          <Trash2 className="mr-1 h-4 w-4" /> {t("common.delete")}
        </Button>
        <div className="flex items-center gap-2">
          {editing && (
            <Button variant="outline" onClick={onClone}>
              <Copy className="mr-1 h-4 w-4" /> {t("common.clone")}
            </Button>
          )}
          <Button variant="outline" onClick={() => navigate("/agents")}>
            {t("common.cancel")}
          </Button>
          <Button variant="outline" onClick={() => save(false)} disabled={saving}>
            {saving ? t("common.saving") : t("common.save")}
          </Button>
          <Button onClick={() => save(true)} disabled={saving}>
            {t("agents.form.saveAndChat")}
          </Button>
        </div>
      </div>

      <DeleteAgentDialog
        open={pendingDelete}
        agent={
          agent
            ? {
                id: agent.id,
                name: agent.name,
                conversation_count: 0,
                attached_skill_count: 0,
              }
            : null
        }
        pending={del.isPending}
        onClose={() => setPendingDelete(false)}
        onConfirm={onDelete}
      />
    </div>
  );
}

function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string;
  hint?: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-baseline justify-between">
        <Label>{label}</Label>
        {hint && <span className="text-xs text-muted-foreground">{hint}</span>}
      </div>
      {children}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
