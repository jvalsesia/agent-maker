import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { MessageSquare } from "lucide-react";
import type { Message, SubagentNotice } from "@/lib/api";
import type { DraftAssistant, SubagentDraft } from "@/hooks/useChat";
import { MessageBubble, SubagentBubble } from "./MessageBubble";
import { Markdown } from "./Markdown";

interface Props {
  messages: Message[];
  loading: boolean;
  error: unknown;
  onReload: () => void;
  /** Live assistant reply being streamed, if any. */
  draft?: DraftAssistant | null;
  /** Live sub-agent replies (F11) streamed before the parent draft. */
  subagentDrafts?: SubagentDraft[];
  /** Inline `@mention` notices for the current turn (overflow / unknown). */
  notices?: SubagentNotice[];
  /** Optimistic user message shown while the first chunk is pending. */
  pendingUser?: string | null;
  /** Retry the trailing errored/stopped assistant turn. */
  onRetryTurn?: () => void;
}

/** Count, for each message index, the contiguous run of sub-agent turns that
 *  immediately precedes a parent assistant turn (its "consulted" count). */
function consultedCounts(messages: Message[]): number[] {
  return messages.map((m, i) => {
    if (m.role !== "assistant" || m.subagent_alias) return 0;
    let n = 0;
    for (let j = i - 1; j >= 0 && messages[j].subagent_alias; j--) n++;
    return n;
  });
}

export function MessageList({
  messages,
  loading,
  error,
  onReload,
  draft,
  subagentDrafts,
  notices,
  pendingUser,
  onRetryTurn,
}: Props) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to the latest content whenever messages, the streamed draft,
  // or the optimistic user bubble change.
  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [messages.length, pendingUser, draft?.content, subagentDrafts?.length]);

  const counts = consultedCounts(messages);

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
    <div ref={scrollRef} className="flex-1 space-y-3 overflow-y-auto p-4">
      {messages.map((m, i) => (
        <MessageBubble
          key={m.id}
          message={m}
          onRetry={onRetryTurn}
          consultedCount={counts[i]}
        />
      ))}

      {pendingUser != null && (
        <div className="flex justify-end">
          <div className="max-w-2xl rounded-md border border-border bg-accent px-3 py-2 text-sm text-accent-foreground">
            <div className="mb-1 text-xs uppercase tracking-wide text-muted-foreground">user</div>
            <p className="whitespace-pre-wrap">{pendingUser}</p>
          </div>
        </div>
      )}

      {notices?.map((n, i) => (
        <p key={`notice-${i}`} className="text-xs text-muted-foreground" role="status">
          {n.code === "unknown"
            ? t("chat.subagent.noticeUnknown", { aliases: n.aliases.join(", ") })
            : t("chat.subagent.noticeOverflow", { aliases: n.aliases.join(", ") })}
        </p>
      ))}

      {subagentDrafts?.map((s) => (
        <SubagentBubble key={s.message_id} alias={s.alias} content={s.content} status={s.status} />
      ))}

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
