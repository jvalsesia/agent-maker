import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AlertTriangle, Copy, Trash2 } from "lucide-react";
import { ApiError, type SkillUpsert, type SkillWarning } from "@/lib/api";
import {
  useCloneSkill,
  useCreateSkill,
  useDeleteSkill,
  useSkill,
  useUpdateSkill,
} from "@/hooks/useSkills";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { DeleteSkillDialog } from "./DeleteSkillDialog";

interface FieldErrors {
  [field: string]: string;
}

export function SkillForm() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const editing = !!id;
  const navigate = useNavigate();

  const existing = useSkill(id);

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [body, setBody] = useState("");
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [warnings, setWarnings] = useState<SkillWarning[]>([]);
  const [pendingDelete, setPendingDelete] = useState(false);

  useEffect(() => {
    const s = existing.data?.skill;
    if (!s) return;
    setName(s.name);
    setDescription(s.description);
    setBody(s.body);
  }, [existing.data?.skill]);

  const create = useCreateSkill();
  const update = useUpdateSkill(id ?? "");
  const clone = useCloneSkill();
  const del = useDeleteSkill();

  const buildBody = (): SkillUpsert => ({
    name: name.trim(),
    description: description.trim(),
    body,
  });

  const handleApiError = (e: unknown) => {
    if (e instanceof ApiError && e.body.error.field) {
      setFieldErrors({ [e.body.error.field]: e.body.error.message });
    } else {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const save = async () => {
    setFieldErrors({});
    setWarnings([]);
    try {
      const resp = editing
        ? await update.mutateAsync(buildBody())
        : await create.mutateAsync(buildBody());
      setWarnings(resp.warnings);
      toast.success(t("skills.form.saved"));
      if (!editing) {
        navigate(`/skills/${resp.skill.id}`, { replace: true });
      }
    } catch (e) {
      handleApiError(e);
    }
  };

  const onClone = async () => {
    if (!id) return;
    try {
      const r = await clone.mutateAsync({ id });
      toast.success(t("skills.form.cloned", { name: r.skill.name }));
      navigate(`/skills/${r.skill.id}`);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onDelete = async () => {
    if (!id) return;
    try {
      await del.mutateAsync(id);
      toast.success(t("skills.form.deleted"));
      navigate("/skills");
    } catch (e) {
      handleApiError(e);
    }
  };

  const bodyWarn = warnings.find((w) => w.field === "body");
  const saving = create.isPending || update.isPending;
  const skill = existing.data?.skill;
  const usingAgents = existing.data?.using_agents ?? [];

  return (
    <div className="mx-auto max-w-3xl pb-24">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            {editing ? skill?.name ?? t("skills.form.fallbackTitle") : t("skills.form.newTitle")}
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            <Link to="/skills" className="underline">
              {t("skills.form.allSkills")}
            </Link>
          </p>
        </div>
      </div>

      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="md:col-span-2 space-y-5">
          <Field label={t("skills.form.name")} hint={`${name.length} / 60`} error={fieldErrors.name}>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("skills.form.namePlaceholder")}
              maxLength={60}
            />
          </Field>

          <Field
            label={t("skills.form.description")}
            hint={`${description.length} / 200`}
            error={fieldErrors.description}
          >
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t("skills.form.descriptionPlaceholder")}
              maxLength={200}
            />
          </Field>

          <Field
            label={t("skills.form.body")}
            hint={t("skills.form.bodyChars", { count: body.length })}
            error={fieldErrors.body}
          >
            <Textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder={t("skills.form.bodyPlaceholder")}
              rows={14}
              className="font-mono text-xs"
            />
            {bodyWarn && (
              <p className="mt-1 flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
                <AlertTriangle className="h-3 w-3" /> {bodyWarn.message}
              </p>
            )}
          </Field>
        </div>

        {editing && (
          <aside className="rounded-md border border-border bg-card/60 p-4 h-fit">
            <Label className="text-xs uppercase tracking-wide text-muted-foreground">
              {t("skills.form.usedBy", { count: usingAgents.length })}
            </Label>
            {usingAgents.length === 0 ? (
              <p className="mt-2 text-sm text-muted-foreground">
                {t("skills.form.noAgentsHint")}
              </p>
            ) : (
              <ul className="mt-2 space-y-1 text-sm">
                {usingAgents.map((a) => (
                  <li key={a.id}>
                    <Link to={`/agents/${a.id}`} className="underline">
                      {a.name}
                    </Link>
                  </li>
                ))}
              </ul>
            )}
          </aside>
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
          <Button variant="outline" onClick={() => navigate("/skills")}>
            {t("common.cancel")}
          </Button>
          <Button onClick={save} disabled={saving}>
            {saving ? t("common.saving") : t("common.save")}
          </Button>
        </div>
      </div>

      <DeleteSkillDialog
        open={pendingDelete}
        skill={
          skill
            ? { id: skill.id, name: skill.name, using_agents: usingAgents }
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
