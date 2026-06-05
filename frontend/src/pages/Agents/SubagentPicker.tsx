import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
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
import { useAgents } from "@/hooks/useAgents";
import type { AttachSubagentInput } from "@/lib/api";

interface Props {
  open: boolean;
  /** The parent agent — excluded from the list (an agent cannot attach itself). */
  parentId: string;
  /** Child ids already attached — shown disabled. */
  alreadyAttachedIds: string[];
  pending: boolean;
  onClose: () => void;
  onConfirm: (input: AttachSubagentInput) => void;
}

export function SubagentPicker({
  open,
  parentId,
  alreadyAttachedIds,
  pending,
  onClose,
  onConfirm,
}: Props) {
  const { t } = useTranslation();
  const { data, isLoading } = useAgents();
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<string | null>(null);
  const [alias, setAlias] = useState("");
  const [description, setDescription] = useState("");

  useEffect(() => {
    if (open) {
      setSearch("");
      setSelected(null);
      setAlias("");
      setDescription("");
    }
  }, [open]);

  const candidates = useMemo(() => {
    const agents = (data?.agents ?? []).filter((a) => a.id !== parentId);
    const q = search.trim().toLowerCase();
    if (!q) return agents;
    return agents.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        (a.preamble ?? "").toLowerCase().includes(q),
    );
  }, [data?.agents, parentId, search]);

  const onSubmit = () => {
    if (!selected) return;
    onConfirm({
      child_id: selected,
      alias: alias.trim() || undefined,
      description: description.trim() || undefined,
    });
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("agents.subagentPicker.title")}</DialogTitle>
          <DialogDescription>{t("agents.subagentPicker.description")}</DialogDescription>
        </DialogHeader>

        <div className="min-w-0 space-y-3">
          <Input
            placeholder={t("agents.subagentPicker.searchPlaceholder")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <div className="max-h-56 overflow-y-auto rounded-md border border-border">
            {isLoading && (
              <p className="px-3 py-4 text-sm text-muted-foreground">{t("common.loading")}</p>
            )}
            {!isLoading && candidates.length === 0 && (
              <p className="px-3 py-4 text-sm text-muted-foreground">
                {t("agents.subagentPicker.noMatch")}
              </p>
            )}
            {candidates.map((a) => {
              const isAttached = alreadyAttachedIds.includes(a.id);
              return (
                <label
                  key={a.id}
                  className={`flex items-start gap-3 border-b border-border px-3 py-2 last:border-b-0 ${
                    isAttached ? "bg-muted/30 opacity-60" : "cursor-pointer"
                  }`}
                >
                  <input
                    type="radio"
                    name="subagent-candidate"
                    className="mt-1"
                    disabled={isAttached}
                    checked={selected === a.id}
                    onChange={() => setSelected(a.id)}
                  />
                  <div className="min-w-0">
                    <div className="truncate text-sm font-medium">{a.name}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {isAttached
                        ? t("agents.subagentPicker.alreadyAttached")
                        : (a.preamble ?? "")}
                    </div>
                  </div>
                </label>
              );
            })}
          </div>

          {selected && (
            <div className="space-y-2 rounded-md border border-border p-3">
              <div className="space-y-1">
                <label className="text-xs font-medium text-muted-foreground">
                  {t("agents.subagentPicker.aliasLabel")}
                </label>
                <Input
                  placeholder={t("agents.subagentPicker.aliasPlaceholder")}
                  value={alias}
                  onChange={(e) => setAlias(e.target.value)}
                />
              </div>
              <div className="space-y-1">
                <label className="text-xs font-medium text-muted-foreground">
                  {t("agents.subagentPicker.descriptionLabel")}
                </label>
                <Input
                  placeholder={t("agents.subagentPicker.descriptionPlaceholder")}
                  maxLength={200}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                />
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={pending}>
            {t("common.cancel")}
          </Button>
          <Button onClick={onSubmit} disabled={pending || !selected}>
            {pending ? t("common.saving") : t("agents.subagentPicker.attach")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
