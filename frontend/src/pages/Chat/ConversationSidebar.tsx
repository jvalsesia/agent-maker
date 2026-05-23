import { useState, useEffect } from "react";
import { Eraser, Pencil, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/cn";
import { ApiError, type Conversation } from "@/lib/api";
import {
  useCreateConversation,
  useDeleteConversation,
  useRenameConversation,
} from "@/hooks/useConversations";
import { useClearMemory } from "@/hooks/useMemory";
import { DeleteConversationDialog } from "./DeleteConversationDialog";
import { ClearMemoryDialog } from "./ClearMemoryDialog";

interface Props {
  agentId: string;
  conversations: Conversation[];
  activeId: string | null;
  onSelect: (id: string) => void;
}

export function ConversationSidebar({ agentId, conversations, activeId, onSelect }: Props) {
  const create = useCreateConversation(agentId);
  const rename = useRenameConversation(agentId);
  const del = useDeleteConversation(agentId);

  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [pendingDelete, setPendingDelete] = useState<Conversation | null>(null);
  const [pendingClear, setPendingClear] = useState<Conversation | null>(null);

  const qc = useQueryClient();
  const clearMemory = useClearMemory(pendingClear?.id ?? "");

  const onConfirmClear = async () => {
    if (!pendingClear) return;
    const id = pendingClear.id;
    try {
      const r = await clearMemory.mutateAsync();
      toast.success(`Cleared ${r.removed} embedded turn${r.removed === 1 ? "" : "s"}`);
      qc.invalidateQueries({ queryKey: ["memory-stats", id] });
      setPendingClear(null);
    } catch (e) {
      toast.error(
        e instanceof ApiError ? e.body.error.message : "Couldn't clear memory",
      );
    }
  };

  const onCreate = async () => {
    try {
      const r = await create.mutateAsync(undefined);
      onSelect(r.conversation.id);
    } catch (e) {
      toast.error(
        e instanceof ApiError ? e.body.error.message : "Couldn't create conversation",
      );
    }
  };

  const startRename = (c: Conversation) => {
    setEditingId(c.id);
    setEditValue(c.title);
  };

  const submitRename = async () => {
    if (!editingId) return;
    const id = editingId;
    const title = editValue.trim();
    if (!title) {
      setEditingId(null);
      return;
    }
    try {
      await rename.mutateAsync({ id, title });
      setEditingId(null);
    } catch (e) {
      toast.error(
        e instanceof ApiError ? e.body.error.message : "Couldn't rename",
      );
    }
  };

  const onConfirmDelete = async () => {
    if (!pendingDelete) return;
    const deletedId = pendingDelete.id;
    try {
      await del.mutateAsync(deletedId);
      setPendingDelete(null);
      if (activeId === deletedId) {
        const next = conversations.find((c) => c.id !== deletedId);
        if (next) onSelect(next.id);
      }
    } catch (e) {
      toast.error(e instanceof ApiError ? e.body.error.message : "Couldn't delete");
    }
  };

  return (
    <aside className="flex w-64 shrink-0 flex-col border-r border-border bg-card/40">
      <div className="border-b border-border p-2">
        <Button className="w-full" size="sm" onClick={onCreate} disabled={create.isPending}>
          <Plus className="mr-1 h-4 w-4" /> New conversation
        </Button>
      </div>
      <div className="flex-1 overflow-y-auto p-1">
        {conversations.map((c) => {
          const active = c.id === activeId;
          const editing = editingId === c.id;
          return (
            <div
              key={c.id}
              className={cn(
                "group flex items-center gap-1 rounded-md px-2 py-2 text-sm",
                active
                  ? "bg-accent text-accent-foreground"
                  : "text-foreground/80 hover:bg-accent/60",
              )}
            >
              {editing ? (
                <RenameRow
                  value={editValue}
                  onChange={setEditValue}
                  onSubmit={submitRename}
                  onCancel={() => setEditingId(null)}
                />
              ) : (
                <>
                  <button
                    type="button"
                    onClick={() => onSelect(c.id)}
                    className="min-w-0 flex-1 truncate text-left"
                  >
                    <div className="truncate font-medium">{c.title}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {c.message_count} msg · {formatRelative(c.last_activity_at)}
                    </div>
                  </button>
                  <div className="flex shrink-0 items-center gap-0.5 opacity-0 group-hover:opacity-100">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() => startRename(c)}
                      aria-label="Rename"
                    >
                      <Pencil className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() => setPendingClear(c)}
                      aria-label="Clear memory"
                    >
                      <Eraser className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 w-7 p-0"
                      onClick={() => setPendingDelete(c)}
                      aria-label="Delete"
                    >
                      <Trash2 className="h-3.5 w-3.5 text-destructive" />
                    </Button>
                  </div>
                </>
              )}
            </div>
          );
        })}
      </div>

      <DeleteConversationDialog
        open={!!pendingDelete}
        conversation={pendingDelete}
        onClose={() => setPendingDelete(null)}
        onConfirm={onConfirmDelete}
        pending={del.isPending}
      />

      <ClearMemoryDialog
        open={!!pendingClear}
        conversationId={pendingClear?.id ?? null}
        conversationTitle={pendingClear?.title ?? ""}
        pending={clearMemory.isPending}
        onClose={() => setPendingClear(null)}
        onConfirm={onConfirmClear}
      />
    </aside>
  );
}

function RenameRow({
  value,
  onChange,
  onSubmit,
  onCancel,
}: {
  value: string;
  onChange: (s: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    // focus is handled via autoFocus on input
  }, []);
  return (
    <Input
      autoFocus
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") onSubmit();
        else if (e.key === "Escape") onCancel();
      }}
      onBlur={onSubmit}
      className="h-7 px-2 py-1 text-sm"
    />
  );
}

function formatRelative(iso: string): string {
  const t = new Date(iso).getTime();
  const diff = Date.now() - t;
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return "just now";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}d ago`;
  return new Date(iso).toLocaleDateString();
}
