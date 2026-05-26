import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface Props {
  open: boolean;
  conversation: { id: string; title: string; message_count: number } | null;
  onClose: () => void;
  onConfirm: () => void;
  pending: boolean;
}

export function DeleteConversationDialog({ open, conversation, onClose, onConfirm, pending }: Props) {
  const { t } = useTranslation();
  return (
    <Dialog open={open} onOpenChange={(o) => (!o ? onClose() : undefined)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("chat.deleteConversation.title")}</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          {conversation ? (
            <>
              <span className="font-medium text-foreground">{conversation.title}</span>{" "}
              {t("chat.deleteConversation.willBeRemoved")}{" "}
              <span className="font-medium text-foreground">
                {t("chat.deleteConversation.messages", { count: conversation.message_count })}
              </span>
              {t("chat.deleteConversation.cannotUndo")}
            </>
          ) : null}
        </p>
        <DialogFooter className="mt-4 gap-2">
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
