import { useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Copy, Plus, Sparkles, Trash2 } from "lucide-react";
import { ApiError, type SkillSort } from "@/lib/api";
import { useCloneSkill, useDeleteSkill, useSkills } from "@/hooks/useSkills";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DeleteSkillDialog } from "./DeleteSkillDialog";

export function SkillsList() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<SkillSort>("name");
  const params = useMemo(
    () => ({
      sort,
      order: sort === "name" ? ("asc" as const) : ("desc" as const),
      q: search || undefined,
    }),
    [sort, search],
  );
  const { data, isLoading } = useSkills(params);
  const clone = useCloneSkill();
  const del = useDeleteSkill();
  const [pendingDelete, setPendingDelete] = useState<{
    id: string;
    name: string;
    using_agents: { id: string; name: string }[];
  } | null>(null);

  const skills = data?.skills ?? [];

  const onClone = async (id: string) => {
    try {
      const r = await clone.mutateAsync({ id });
      toast.success(t("skills.list.cloned", { name: r.skill.name }));
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const requestDelete = async (id: string, name: string) => {
    try {
      // Pull the detailed record so the confirmation lists using agents.
      const detail = await api.getSkill(id);
      setPendingDelete({ id, name, using_agents: detail.using_agents });
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const onConfirmDelete = async () => {
    if (!pendingDelete) return;
    try {
      await del.mutateAsync(pendingDelete.id);
      toast.success(t("skills.list.deleted", { name: pendingDelete.name }));
      setPendingDelete(null);
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <div className="mx-auto max-w-4xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">{t("skills.list.title")}</h1>
          <p className="text-sm text-muted-foreground mt-1">{t("skills.list.subtitle")}</p>
        </div>
        <Button onClick={() => navigate("/skills/new")}>
          <Plus className="mr-1 h-4 w-4" /> {t("skills.list.newButton")}
        </Button>
      </div>

      <div className="mt-6 flex items-center gap-3">
        <Input
          placeholder={t("skills.list.searchPlaceholder")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="max-w-sm"
        />
        <Select value={sort} onValueChange={(v) => setSort(v as SkillSort)}>
          <SelectTrigger className="w-44">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="name">{t("skills.list.sortName")}</SelectItem>
            <SelectItem value="attached">{t("skills.list.sortAttached")}</SelectItem>
            <SelectItem value="created">{t("skills.list.sortCreated")}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="mt-4 space-y-2">
        {isLoading ? (
          <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
        ) : skills.length === 0 ? (
          <div className="rounded-md border border-dashed border-border p-10 text-center">
            <Sparkles className="mx-auto mb-3 h-8 w-8 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">{t("skills.list.empty")}</p>
            <Button className="mt-4" onClick={() => navigate("/skills/new")}>
              <Plus className="mr-1 h-4 w-4" /> {t("skills.list.createButton")}
            </Button>
          </div>
        ) : (
          skills.map((s) => (
            <div
              key={s.id}
              className="flex items-center justify-between rounded-md border border-border bg-card/60 p-4 hover:bg-card transition-colors"
            >
              <Link to={`/skills/${s.id}`} className="min-w-0 flex-1 pr-4">
                <div className="flex items-center gap-2">
                  <span className="truncate font-medium">{s.name}</span>
                  <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                    {t("skills.list.agentCount", { count: s.attached_agent_count })}
                  </span>
                </div>
                <p className="mt-1 truncate text-sm text-muted-foreground">{s.description}</p>
              </Link>
              <div className="flex items-center gap-1">
                <Button variant="ghost" size="sm" onClick={() => onClone(s.id)}>
                  <Copy className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => requestDelete(s.id, s.name)}
                >
                  <Trash2 className="h-4 w-4 text-destructive" />
                </Button>
              </div>
            </div>
          ))
        )}
      </div>

      <DeleteSkillDialog
        open={!!pendingDelete}
        skill={pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={onConfirmDelete}
        pending={del.isPending}
      />
    </div>
  );
}
