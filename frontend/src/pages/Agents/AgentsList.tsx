import { useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Bot, Copy, CornerDownRight, MessageSquare, Plus, Trash2 } from "lucide-react";
import { ApiError, type AgentSort } from "@/lib/api";
import { useAgents, useCloneAgent, useDeleteAgent } from "@/hooks/useAgents";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DeleteAgentDialog } from "./DeleteAgentDialog";

export function AgentsList() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<AgentSort>("name");
  const params = useMemo(
    () => ({ sort, order: sort === "name" ? ("asc" as const) : ("desc" as const), q: search || undefined }),
    [sort, search],
  );
  const { data, isLoading } = useAgents(params);
  const clone = useCloneAgent();
  const del = useDeleteAgent();
  const [pendingDelete, setPendingDelete] = useState<{
    id: string;
    name: string;
    conversation_count: number;
    attached_skill_count: number;
  } | null>(null);

  const agents = data?.agents ?? [];

  const onClone = async (id: string) => {
    try {
      const r = await clone.mutateAsync({ id });
      toast.success(t("agents.list.cloned", { name: r.agent.name }));
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  const onConfirmDelete = async () => {
    if (!pendingDelete) return;
    try {
      await del.mutateAsync(pendingDelete.id);
      toast.success(t("agents.list.deleted", { name: pendingDelete.name }));
      setPendingDelete(null);
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : String(e));
    }
  };

  return (
    <div className="mx-auto max-w-4xl">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">{t("agents.list.title")}</h1>
          <p className="text-sm text-muted-foreground mt-1">{t("agents.list.subtitle")}</p>
        </div>
        <Button onClick={() => navigate("/agents/new")}>
          <Plus className="mr-1 h-4 w-4" /> {t("agents.list.newButton")}
        </Button>
      </div>

      <div className="mt-6 flex items-center gap-3">
        <Input
          placeholder={t("agents.list.searchPlaceholder")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="max-w-sm"
        />
        <Select value={sort} onValueChange={(v) => setSort(v as AgentSort)}>
          <SelectTrigger className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="name">{t("agents.list.sortName")}</SelectItem>
            <SelectItem value="last_used">{t("agents.list.sortLastUsed")}</SelectItem>
            <SelectItem value="created">{t("agents.list.sortCreated")}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="mt-4 space-y-2">
        {isLoading ? (
          <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
        ) : agents.length === 0 ? (
          <div className="rounded-md border border-dashed border-border p-10 text-center">
            <Bot className="mx-auto mb-3 h-8 w-8 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">{t("agents.list.empty")}</p>
            <Button className="mt-4" onClick={() => navigate("/agents/new")}>
              <Plus className="mr-1 h-4 w-4" /> {t("agents.list.createButton")}
            </Button>
          </div>
        ) : (
          agents.map((a) => (
            <div
              key={a.id}
              className="flex items-center justify-between rounded-md border border-border bg-card/60 p-4 hover:bg-card transition-colors"
            >
              <Link to={`/agents/${a.id}`} className="min-w-0 flex-1 pr-4">
                <div className="flex items-center gap-2">
                  <span className="truncate font-medium">{a.name}</span>
                  <span className="rounded bg-muted px-1.5 py-0.5 text-xs font-mono text-muted-foreground">
                    {a.provider}:{a.model}
                  </span>
                </div>
                {a.preamble && (
                  <p className="mt-1 truncate text-sm text-muted-foreground">{a.preamble}</p>
                )}
                {a.subagents.length > 0 && (
                  <div
                    className="mt-1.5 flex flex-wrap items-center gap-1.5"
                    aria-label={t("agents.list.subagentsLabel")}
                  >
                    <CornerDownRight className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                    {a.subagents.map((s) => (
                      <span
                        key={s.alias}
                        title={s.name}
                        className="rounded bg-muted px-1.5 py-0.5 text-xs font-mono text-muted-foreground"
                      >
                        @{s.alias}
                      </span>
                    ))}
                  </div>
                )}
              </Link>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => navigate(`/agents/${a.id}/chat`)}
                  aria-label={t("agents.list.openChat")}
                >
                  <MessageSquare className="h-4 w-4" />
                </Button>
                <Button variant="ghost" size="sm" onClick={() => onClone(a.id)}>
                  <Copy className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    setPendingDelete({
                      id: a.id,
                      name: a.name,
                      conversation_count: a.conversation_count,
                      attached_skill_count: a.attached_skill_count,
                    })
                  }
                >
                  <Trash2 className="h-4 w-4 text-destructive" />
                </Button>
              </div>
            </div>
          ))
        )}
      </div>

      <DeleteAgentDialog
        open={!!pendingDelete}
        agent={pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={onConfirmDelete}
        pending={del.isPending}
      />
    </div>
  );
}
