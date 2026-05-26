import { useTranslation } from "react-i18next";
import { MessageSquare } from "lucide-react";
import type { Message } from "@/lib/api";
import type { DraftAssistant } from "@/hooks/useChat";
import { MessageBubble } from "./MessageBubble";
import { Markdown } from "./Markdown";

interface Props {
  messages: Message[];
  loading: boolean;
  error: unknown;
  onReload: () => void;
  /** Live assistant reply being streamed, if any. */
  draft?: DraftAssistant | null;
  /** Optimistic user message shown while the first chunk is pending. */
  pendingUser?: string | null;
  /** Retry the trailing errored/stopped assistant turn. */
  onRetryTurn?: () => void;
}

export function MessageList({
  messages,
  loading,
  error,
  onReload,
  draft,
  pendingUser,
  onRetryTurn,
}: Props) {
  const { t } = useTranslation();
  if (loading) {
    return <p className="p-6 text-sm text-muted-foreground">{t("chat.loadingMessages")}</p>;
  }
  if (error) {
    return (
      <div className="p-6 text-sm">
        <p className="text-destructive">{t("chat.loadError")}</p>
        <button onClick={onReload} className="mt-2 text-foreground underline">
          {t("common.retry")}
        </button>
      </div>
    );
  }

  const empty = messages.length === 0 && !pendingUser && !draft;
  if (empty) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center p-10 text-center text-sm text-muted-foreground">
        <MessageSquare className="mb-3 h-8 w-8" />
        <p>{t("chat.emptyTitle")}</p>
        <p className="mt-1 max-w-sm">{t("chat.emptyHint")}</p>
      </div>
    );
  }

  return (
    <div className="flex-1 space-y-3 overflow-y-auto p-4">
      {messages.map((m) => (
        <MessageBubble key={m.id} message={m} onRetry={onRetryTurn} />
      ))}

      {pendingUser != null && (
        <div className="flex justify-end">
          <div className="max-w-2xl rounded-md border border-border bg-accent px-3 py-2 text-sm text-accent-foreground">
            <div className="mb-1 text-xs uppercase tracking-wide text-muted-foreground">user</div>
            <p className="whitespace-pre-wrap">{pendingUser}</p>
          </div>
        </div>
      )}

      {draft && (
        <div className="flex justify-start">
          <div className="max-w-2xl rounded-md border border-border bg-card/60 px-3 py-2 text-sm">
            <div className="mb-1 text-xs uppercase tracking-wide text-muted-foreground">
              assistant
            </div>
            {draft.degraded && (
              <p className="mb-1 text-xs text-muted-foreground">{t("chat.memoryUnavailable")}</p>
            )}
            {draft.content ? <Markdown>{draft.content}</Markdown> : <span className="text-muted-foreground">▍</span>}
            {draft.content && <span className="animate-pulse">▍</span>}
          </div>
        </div>
      )}
    </div>
  );
}
