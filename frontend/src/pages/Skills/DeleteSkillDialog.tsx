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
          <DialogTitle>Delete skill</DialogTitle>
          <DialogDescription>
            This will permanently remove the skill
            {skill ? ` “${skill.name}”` : ""} from your workspace.
          </DialogDescription>
        </DialogHeader>
        {skill && skill.using_agents.length > 0 ? (
          <div className="text-sm text-muted-foreground space-y-1">
            <p>The following {skill.using_agents.length} agent(s) currently use this skill:</p>
            <ul className="list-disc list-inside">
              {skill.using_agents.map((a) => (
                <li key={a.id}>{a.name}</li>
              ))}
            </ul>
            <p>They will be detached automatically.</p>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            No agents currently use this skill.
          </p>
        )}
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
