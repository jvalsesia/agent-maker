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
  return (
    <Dialog open={open} onOpenChange={(o) => (!o ? onClose() : undefined)}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete conversation?</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          {conversation ? (
            <>
              <span className="font-medium text-foreground">{conversation.title}</span>{" "}
              will be removed along with{" "}
              <span className="font-medium text-foreground">
                {conversation.message_count} message
                {conversation.message_count === 1 ? "" : "s"}
              </span>
              . This cannot be undone.
            </>
          ) : null}
        </p>
        <DialogFooter className="mt-4 gap-2">
          <Button variant="ghost" onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm} disabled={pending}>
            {pending ? "Deleting…" : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
