import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, X } from "lucide-react";
import { toast } from "sonner";
import { ApiError, type AttachSubagentInput } from "@/lib/api";
import {
  useAttachSubagent,
  useAttachedSubagents,
  useDetachSubagent,
} from "@/hooks/useSubagents";
import { SubagentPicker } from "@/pages/Agents/SubagentPicker";

interface Props {
  /** The current chat agent — the parent whose roster this bar manages. */
  agentId: string;
  /** Insert the given @handle into the composer at the caret (and refocus). */
  onPick: (alias: string) => void;
}

/**
 * Interactive sub-agents bar above the composer (F11). Lists the current agent's
 * attached sub-agents as @handle chips (each clickable to summon and with a detach
 * control), and an "Attach sub-agent" action reusing the agent-detail picker. On a
 * successful attach the freshly attached @handle is inserted into the composer via
 * `onPick`. The bar stays visible even with no sub-agents so the attach control is
 * always reachable from chat.
 */
export function SubagentBar({ agentId, onPick }: Props) {
  const { t } = useTranslation();
  const { data } = useAttachedSubagents(agentId);
  const attach = useAttachSubagent(agentId);
  const detach = useDetachSubagent(agentId);
  const [pickerOpen, setPickerOpen] = useState(false);

  const attached = data?.attached ?? [];
  const busy = attach.isPending || detach.isPending;

  const handleApiError = (e: unknown) => {
    toast.error(e instanceof ApiError ? e.body.error.message : String(e));
  };

  // Insert the server-returned (canonical/defaulted) alias on success, so nothing
  // is inserted when the attach is rejected (cycle / cap / bad alias).
  const onAttachConfirm = async (input: AttachSubagentInput) => {
    try {
      const res = await attach.mutateAsync(input);
      setPickerOpen(false);
      onPick(res.attached.alias);
      toast.success(t("agents.subagentsPanel.attached"));
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

  return (
    <div className="flex flex-wrap items-center gap-1.5 border-t border-border px-4 pt-2 text-xs text-muted-foreground">
      <span className="shrink-0">{t("chat.subagentBar.label")}</span>
      {attached.length === 0 && (
        <span className="text-muted-foreground/70">{t("chat.subagentBar.empty")}</span>
      )}
      {attached.map((s) => (
        <span
          key={s.child_id}
          title={s.description ? `${s.name} — ${s.description}` : s.name}
          className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 font-mono text-foreground"
        >
          <button type="button" className="hover:underline" onClick={() => onPick(s.alias)}>
            @{s.alias}
          </button>
          <button
            type="button"
            aria-label={t("chat.subagentBar.detach", { name: s.name })}
            className="rounded text-muted-foreground hover:text-foreground disabled:opacity-40"
            onClick={() => onDetach(s.child_id)}
            disabled={busy}
          >
            <X className="h-3 w-3" />
          </button>
        </span>
      ))}
      <button
        type="button"
        className="inline-flex items-center gap-1 rounded border border-border px-1.5 py-0.5 hover:bg-muted disabled:opacity-40"
        onClick={() => setPickerOpen(true)}
        disabled={busy || attached.length >= 10}
      >
        <Plus className="h-3 w-3" /> {t("chat.subagentBar.attach")}
      </button>

      <SubagentPicker
        open={pickerOpen}
        parentId={agentId}
        alreadyAttachedIds={attached.map((a) => a.child_id)}
        pending={attach.isPending}
        onClose={() => setPickerOpen(false)}
        onConfirm={onAttachConfirm}
      />
    </div>
  );
}
