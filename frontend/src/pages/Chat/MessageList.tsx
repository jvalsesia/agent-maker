import { MessageSquare } from "lucide-react";
import { cn } from "@/lib/cn";
import type { Message } from "@/lib/api";

interface Props {
  messages: Message[];
  loading: boolean;
  error: unknown;
  onRetry: () => void;
}

export function MessageList({ messages, loading, error, onRetry }: Props) {
  if (loading) {
    return <p className="p-6 text-sm text-muted-foreground">Loading messages…</p>;
  }
  if (error) {
    return (
      <div className="p-6 text-sm">
        <p className="text-destructive">Couldn't load messages.</p>
        <button onClick={onRetry} className="mt-2 text-foreground underline">
          Retry
        </button>
      </div>
    );
  }
  if (messages.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center p-10 text-center text-sm text-muted-foreground">
        <MessageSquare className="mb-3 h-8 w-8" />
        <p>No messages yet.</p>
        <p className="mt-1 max-w-sm">
          Send your first message once the chat runtime ships (F07).
        </p>
      </div>
    );
  }
  return (
    <div className="flex-1 space-y-3 overflow-y-auto p-4">
      {messages.map((m) => (
        <Bubble key={m.id} message={m} />
      ))}
    </div>
  );
}

function Bubble({ message }: { message: Message }) {
  const isUser = message.role === "user";
  return (
    <div className={cn("flex", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "max-w-2xl rounded-md border border-border px-3 py-2 text-sm whitespace-pre-wrap",
          isUser ? "bg-accent text-accent-foreground" : "bg-card/60",
        )}
      >
        <div className="mb-1 text-xs uppercase tracking-wide text-muted-foreground">
          {message.role}
          {message.status !== "complete" && ` · ${message.status}`}
        </div>
        {message.content}
      </div>
    </div>
  );
}
