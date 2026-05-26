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
  skill: {
    id: string;
    name: string;
    using_agents: { id: string; name: string }[];
  } | null;
  pending: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export function DeleteSkillDialog({ open, skill, pending, onClose, onConfirm }: Props) {
  const { t } = useTranslation();
  // PRD requirement: background-click must not dismiss when the skill is attached.
  // onOpenChange fires for ESC and outside-click; we ignore outside-click here.
  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) onClose();
      }}
    >
      <DialogContent
        onPointerDownOutside={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>{t("skills.deleteDialog.title")}</DialogTitle>
          <DialogDescription>
            {t("skills.deleteDialog.body", { name: skill ? `“${skill.name}”` : "" })}
          </DialogDescription>
        </DialogHeader>
        {skill && skill.using_agents.length > 0 ? (
          <div className="text-sm text-muted-foreground space-y-1">
            <p>{t("skills.deleteDialog.usingAgents", { count: skill.using_agents.length })}</p>
            <ul className="list-disc list-inside">
              {skill.using_agents.map((a) => (
                <li key={a.id}>{a.name}</li>
              ))}
            </ul>
            <p>{t("skills.deleteDialog.willDetach")}</p>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            {t("skills.deleteDialog.noAgents")}
          </p>
        )}
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
