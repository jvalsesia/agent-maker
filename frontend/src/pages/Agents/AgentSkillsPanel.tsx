import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, Plus, X } from "lucide-react";
import { toast } from "sonner";
import { ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  useAttachedSkills,
  useDetachSkill,
  useReplaceAttachedSkills,
} from "@/hooks/useAgentSkills";
import { AttachSkillsDialog } from "./AttachSkillsDialog";
import { ComposedPromptIndicator } from "./ComposedPromptIndicator";

export function AgentSkillsPanel({ agentId }: { agentId: string }) {
  const { t } = useTranslation();
  const { data, isLoading } = useAttachedSkills(agentId);
  const replace = useReplaceAttachedSkills(agentId);
  const detach = useDetachSkill(agentId);

  const [pickerOpen, setPickerOpen] = useState(false);

  const attached = data?.attached ?? [];

  const handleApiError = (e: unknown) => {
    toast.error(e instanceof ApiError ? e.body.error.message : String(e));
  };

  const moveTo = async (from: number, to: number) => {
    if (to < 0 || to >= attached.length || from === to) return;
    const ids = attached.map((a) => a.skill_id);
    const [moved] = ids.splice(from, 1);
    ids.splice(to, 0, moved);
    try {
      await replace.mutateAsync(ids);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onDetach = async (skillId: string) => {
    try {
      await detach.mutateAsync(skillId);
    } catch (e) {
      handleApiError(e);
    }
  };

  const onAttachConfirm = async (mergedIds: string[]) => {
    try {
      const resp = await replace.mutateAsync(mergedIds);
      setPickerOpen(false);
      const warn = resp.warnings?.[0];
      if (warn) {
        toast.warning(warn.message);
      } else {
        toast.success(t("agents.skillsPanel.updated"));
      }
    } catch (e) {
      handleApiError(e);
    }
  };

  return (
    <div className="rounded-md border border-border">
      <div className="border-b border-border px-4 py-3 text-sm font-medium">
        {t("agents.skillsPanel.title")}
      </div>
      <div className="space-y-1 px-2 py-2">
        {isLoading && (
          <p className="px-2 py-2 text-xs text-muted-foreground">{t("common.loading")}</p>
        )}
        {!isLoading && attached.length === 0 && (
          <p className="px-2 py-2 text-xs text-muted-foreground">
            {t("agents.skillsPanel.empty")}
          </p>
        )}
        {attached.map((row, idx) => (
          <div
            key={row.skill_id}
            className="flex items-center gap-2 rounded px-2 py-1.5 hover:bg-muted/50"
          >
            <div className="flex flex-col">
              <button
                type="button"
                className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-30"
                aria-label={t("agents.skillsPanel.moveUp", { name: row.name })}
                onClick={() => moveTo(idx, idx - 1)}
                disabled={idx === 0 || replace.isPending}
              >
                <ArrowUp className="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-30"
                aria-label={t("agents.skillsPanel.moveDown", { name: row.name })}
                onClick={() => moveTo(idx, idx + 1)}
                disabled={idx === attached.length - 1 || replace.isPending}
              >
                <ArrowDown className="h-3.5 w-3.5" />
              </button>
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-sm font-medium">{row.name}</div>
              <div className="truncate text-xs text-muted-foreground">
                {row.description}
              </div>
            </div>
            <Button
              size="sm"
              variant="ghost"
              aria-label={t("agents.skillsPanel.detach", { name: row.name })}
              onClick={() => onDetach(row.skill_id)}
              disabled={detach.isPending || replace.isPending}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        ))}
      </div>
      <div className="flex items-center justify-between gap-3 border-t border-border px-4 py-3">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setPickerOpen(true)}
          disabled={replace.isPending}
        >
          <Plus className="mr-1 h-4 w-4" /> {t("agents.skillsPanel.attach")}
        </Button>
        <ComposedPromptIndicator agentId={agentId} />
      </div>

      <AttachSkillsDialog
        open={pickerOpen}
        alreadyAttachedIds={attached.map((a) => a.skill_id)}
        pending={replace.isPending}
        onClose={() => setPickerOpen(false)}
        onConfirm={onAttachConfirm}
      />
    </div>
  );
}
