import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
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
      toast.success("Skill saved");
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
      toast.success(`Cloned as ${r.skill.name}`);
      navigate(`/skills/${r.skill.id}`);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onDelete = async () => {
    if (!id) return;
    try {
      await del.mutateAsync(id);
      toast.success("Deleted");
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
            {editing ? skill?.name ?? "Skill" : "New skill"}
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            <Link to="/skills" className="underline">
              All skills
            </Link>
          </p>
        </div>
      </div>

      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="md:col-span-2 space-y-5">
          <Field label="Name" hint={`${name.length} / 60`} error={fieldErrors.name}>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Concise Replies"
              maxLength={60}
            />
          </Field>

          <Field
            label="Description"
            hint={`${description.length} / 200`}
            error={fieldErrors.description}
          >
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="One-line summary shown in pickers"
              maxLength={200}
            />
          </Field>

          <Field
            label="Instruction body"
            hint={`${body.length} chars`}
            error={fieldErrors.body}
          >
            <Textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Prefer short sentences. Avoid filler…"
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
              Used by {usingAgents.length} agent{usingAgents.length === 1 ? "" : "s"}
            </Label>
            {usingAgents.length === 0 ? (
              <p className="mt-2 text-sm text-muted-foreground">
                Attach this skill to an agent from the agent's detail page.
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
          <Trash2 className="mr-1 h-4 w-4" /> Delete
        </Button>
        <div className="flex items-center gap-2">
          {editing && (
            <Button variant="outline" onClick={onClone}>
              <Copy className="mr-1 h-4 w-4" /> Clone
            </Button>
          )}
          <Button variant="outline" onClick={() => navigate("/skills")}>
            Cancel
          </Button>
          <Button onClick={save} disabled={saving}>
            {saving ? "Saving…" : "Save"}
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
