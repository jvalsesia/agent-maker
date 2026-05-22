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
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete agent</DialogTitle>
          <DialogDescription>
            This will permanently remove the agent
            {agent ? ` “${agent.name}”` : ""} from your workspace.
          </DialogDescription>
        </DialogHeader>
        <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
          <li>{agent?.conversation_count ?? 0} conversation(s) will be deleted</li>
          <li>{agent?.attached_skill_count ?? 0} attached skill(s) will be detached</li>
        </ul>
        <DialogFooter>
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
