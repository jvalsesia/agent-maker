import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, Plus, Sparkles, X } from "lucide-react";
import { toast } from "sonner";
import { ApiError, type AttachSubagentInput } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  useAttachSubagent,
  useAttachedSubagents,
  useDetachSubagent,
  useReorderSubagents,
} from "@/hooks/useSubagents";
import { SubagentPicker } from "./SubagentPicker";
import { NewSubagentDialog } from "./NewSubagentDialog";

export function SubagentsSection({ agentId }: { agentId: string }) {
  const { t } = useTranslation();
  const { data, isLoading } = useAttachedSubagents(agentId);
  const attach = useAttachSubagent(agentId);
  const detach = useDetachSubagent(agentId);
  const reorder = useReorderSubagents(agentId);

  const [pickerOpen, setPickerOpen] = useState(false);
  const [newOpen, setNewOpen] = useState(false);

  const attached = data?.attached ?? [];
  const busy = attach.isPending || detach.isPending || reorder.isPending;

  const handleApiError = (e: unknown) => {
    toast.error(e instanceof ApiError ? e.body.error.message : String(e));
  };

  const moveTo = async (from: number, to: number) => {
    if (to < 0 || to >= attached.length || from === to) return;
    const ids = attached.map((a) => a.child_id);
    const [moved] = ids.splice(from, 1);
    ids.splice(to, 0, moved);
    try {
      await reorder.mutateAsync(ids);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onDetach = async (childId: string) => {
    try {
      await detach.mutateAsync(childId);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onAttachConfirm = async (input: AttachSubagentInput) => {
    try {
      await attach.mutateAsync(input);
      setPickerOpen(false);
      toast.success(t("agents.subagentsPanel.attached"));
    } catch (e) {
      handleApiError(e);
    }
  };

  const onNewCreated = async (childId: string, alias: string | undefined) => {
    try {
      await attach.mutateAsync({ child_id: childId, alias });
      setNewOpen(false);
      toast.success(t("agents.subagentsPanel.attached"));
    } catch (e) {
      handleApiError(e);
    }
  };

  return (
    <div className="rounded-md border border-border">
      <div className="border-b border-border px-4 py-3 text-sm font-medium">
        {t("agents.subagentsPanel.title")}
      </div>
      <div className="space-y-1 px-2 py-2">
        {isLoading && (
          <p className="px-2 py-2 text-xs text-muted-foreground">{t("common.loading")}</p>
        )}
        {!isLoading && attached.length === 0 && (
          <p className="px-2 py-2 text-xs text-muted-foreground">
            {t("agents.subagentsPanel.empty")}
          </p>
        )}
        {attached.map((row, idx) => (
          <div
            key={row.child_id}
            className="flex items-center gap-2 rounded px-2 py-1.5 hover:bg-muted/50"
          >
            <div className="flex flex-col">
              <button
                type="button"
                className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-30"
                aria-label={t("agents.subagentsPanel.moveUp", { name: row.name })}
                onClick={() => moveTo(idx, idx - 1)}
                disabled={idx === 0 || busy}
              >
                <ArrowUp className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-30"
                aria-label={t("agents.subagentsPanel.moveDown", { name: row.name })}
                onClick={() => moveTo(idx, idx + 1)}
                disabled={idx === attached.length - 1 || busy}
              >
                <ArrowDown className="h-3.5 w-3.5" />
              </button>
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{row.name}</span>
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                  @{row.alias}
                </code>
              </div>
              {row.description && (
                <div className="truncate text-xs text-muted-foreground">{row.description}</div>
              )}
            </div>
            <Button
              size="sm"
              variant="ghost"
              aria-label={t("agents.subagentsPanel.detach", { name: row.name })}
              onClick={() => onDetach(row.child_id)}
              disabled={busy}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        ))}
      </div>
      <div className="flex items-center gap-3 border-t border-border px-4 py-3">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPickerOpen(true)}
          disabled={busy || attached.length >= 10}
        >
          <Plus className="mr-1 h-4 w-4" /> {t("agents.subagentsPanel.attach")}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setNewOpen(true)}
          disabled={busy || attached.length >= 10}
        >
          <Sparkles className="mr-1 h-4 w-4" /> {t("agents.subagentsPanel.newSubagent")}
        </Button>
      </div>

      <SubagentPicker
        open={pickerOpen}
        parentId={agentId}
        alreadyAttachedIds={attached.map((a) => a.child_id)}
        pending={attach.isPending}
        onClose={() => setPickerOpen(false)}
        onConfirm={onAttachConfirm}
      />
      <NewSubagentDialog
        open={newOpen}
        pending={attach.isPending}
        onClose={() => setNewOpen(false)}
        onCreated={onNewCreated}
      />
    </div>
  );
}
