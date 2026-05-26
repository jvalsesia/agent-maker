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

interface Props {
  open: boolean;
  agent: {
    id: string;
    name: string;
    conversation_count: number;
    attached_skill_count: number;
  } | null;
  pending: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export function DeleteAgentDialog({ open, agent, pending, onClose, onConfirm }: Props) {
  const { t } = useTranslation();
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("agents.deleteDialog.title")}</DialogTitle>
          <DialogDescription>
            {t("agents.deleteDialog.body", { name: agent ? `“${agent.name}”` : "" })}
          </DialogDescription>
        </DialogHeader>
        <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
          <li>{t("agents.deleteDialog.conversations", { count: agent?.conversation_count ?? 0 })}</li>
          <li>{t("agents.deleteDialog.skills", { count: agent?.attached_skill_count ?? 0 })}</li>
        </ul>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={pending}>
            {t("common.cancel")}
          </Button>
          <Button variant="destructive" onClick={onConfirm} disabled={pending}>
            {pending ? t("common.deleting") : t("common.delete")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
