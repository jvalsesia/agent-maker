import { useTranslation, Trans } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useMemoryStats } from "@/hooks/useMemory";

interface Props {
  open: boolean;
  conversationId: string | null;
  conversationTitle: string;
  pending: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export function ClearMemoryDialog({
  open,
  conversationId,
  conversationTitle,
  pending,
  onClose,
  onConfirm,
}: Props) {
  const { t } = useTranslation();
  const stats = useMemoryStats(conversationId ?? undefined, open);
  const count = stats.data?.embedded;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("chat.clearMemory.title")}</DialogTitle>
          <DialogDescription>
            <Trans
              i18nKey="chat.clearMemory.description"
              values={{ title: conversationTitle }}
              components={{ strong: <strong /> }}
            />
          </DialogDescription>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          {t("chat.clearMemory.embeddedTurns")}{" "}
          {stats.isLoading
            ? "…"
            : stats.isError
              ? "—"
              : (count ?? 0)}
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={pending}>
            {t("common.cancel")}
          </Button>
          <Button onClick={onConfirm} disabled={pending}>
            {pending ? t("chat.clearMemory.clearing") : t("chat.clearMemory.clearButton")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
