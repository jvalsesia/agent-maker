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
  const stats = useMemoryStats(conversationId ?? undefined, open);
  const count = stats.data?.embedded;

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Clear long-term memory</DialogTitle>
          <DialogDescription>
            Remove embeddings for <strong>{conversationTitle}</strong>. The
            messages themselves stay; only the semantic recall index is wiped.
          </DialogDescription>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Embedded turns:{" "}
          {stats.isLoading
            ? "…"
            : stats.isError
              ? "—"
              : (count ?? 0)}
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button onClick={onConfirm} disabled={pending}>
            {pending ? "Clearing…" : "Clear memory"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
