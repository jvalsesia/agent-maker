import { useEffect, useMemo, useState } from "react";
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
import { useSkills } from "@/hooks/useSkills";

interface Props {
  open: boolean;
  alreadyAttachedIds: string[];
  pending: boolean;
  onClose: () => void;
  onConfirm: (mergedIds: string[]) => void;
}

export function AttachSkillsDialog({
  open,
  alreadyAttachedIds,
  pending,
  onClose,
  onConfirm,
}: Props) {
  const { data, isLoading } = useSkills();
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set(alreadyAttachedIds));

  useEffect(() => {
    if (open) setSelected(new Set(alreadyAttachedIds));
  }, [open, alreadyAttachedIds]);

  const filtered = useMemo(() => {
    const skills = data?.skills ?? [];
    const q = search.trim().toLowerCase();
    if (!q) return skills;
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q),
    );
  }, [data?.skills, search]);

  const toggle = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const onSubmit = () => {
    // Preserve the existing order for already-attached skills, then append
    // newly-checked skills in alphabetical name order.
    const existing = alreadyAttachedIds.filter((id) => selected.has(id));
    const existingSet = new Set(existing);
    const newly = (data?.skills ?? [])
      .filter((s) => selected.has(s.id) && !existingSet.has(s.id))
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((s) => s.id);
    onConfirm([...existing, ...newly]);
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Attach skills</DialogTitle>
          <DialogDescription>
            Select skills to attach to this agent. Skills already attached are pre-checked.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <Input
            placeholder="Search by name or description"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <div className="max-h-72 overflow-y-auto rounded-md border border-border">
            {isLoading && (
              <p className="px-3 py-4 text-sm text-muted-foreground">Loading…</p>
            )}
            {!isLoading && filtered.length === 0 && (
              <p className="px-3 py-4 text-sm text-muted-foreground">
                No skills match.
              </p>
            )}
            {filtered.map((s) => {
              const isAttached = alreadyAttachedIds.includes(s.id);
              return (
                <label
                  key={s.id}
                  className={`flex cursor-pointer items-start gap-3 border-b border-border px-3 py-2 last:border-b-0 ${
                    isAttached ? "bg-muted/30" : ""
                  }`}
                >
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={selected.has(s.id)}
                    onChange={() => toggle(s.id)}
                  />
                  <div className="min-w-0">
                    <div className="text-sm font-medium">{s.name}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {s.description}
                    </div>
                  </div>
                </label>
              );
            })}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button onClick={onSubmit} disabled={pending}>
            {pending ? "Saving…" : "Attach selected"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
