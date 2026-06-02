import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { Message } from "@/lib/api";
import { Markdown } from "./Markdown";
import { RecalledTurns } from "./RecalledTurns";

interface Props {
  message: Message;
  /** Retry an errored/stopped assistant turn. */
  onRetry?: () => void;
  /** Number of sub-agents consulted for this (parent) turn — shows a chip. */
  consultedCount?: number;
}

/** A persisted chat message: markdown for assistant turns, plain text for the
 *  user, plus a model/token chip, status indicators, retry, and recalled turns. */
export function MessageBubble({ message, onRetry, consultedCount }: Props) {
  const { t } = useTranslation();
  const isUser = message.role === "user";
  const errored = message.status === "error";

  // F11: a delegated sub-agent turn renders as a subordinate, labeled bubble.
  if (message.subagent_alias) {
    return (
      <SubagentBubble
        alias={message.subagent_alias}
        content={message.content}
        status={errored ? "error" : "complete"}
      />
    );
  }

  return (
    <div className={cn("flex", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "max-w-2xl rounded-md border px-3 py-2 text-sm",
          isUser ? "bg-accent text-accent-foreground" : "bg-card/60",
          errored ? "border-destructive" : "border-border",
        )}
      >
        <div className="mb-1 flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground">
          <span>{message.role}</span>
          {message.status !== "complete" && <span>· {message.status}</span>}
        </div>

        {isUser ? (
          <p className="whitespace-pre-wrap">{message.content}</p>
        ) : errored && !message.content ? (
          <p className="text-destructive">{t("chat.bubble.providerFailed")}</p>
        ) : (
          <Markdown>{message.content}</Markdown>
        )}

        {!isUser && (message.model || message.token_count != null) && (
          <div className="mt-2 text-[11px] text-muted-foreground">
            {message.model}
            {message.token_count != null && ` · ${t("chat.bubble.tokens", { count: message.token_count })}`}
          </div>
        )}

        {!isUser && consultedCount != null && consultedCount > 0 && (
          <div className="mt-2 inline-flex items-center rounded-full border border-border bg-muted px-2 py-0.5 text-[11px] text-muted-foreground">
            {t("chat.subagent.consulted", { count: consultedCount })}
          </div>
        )}

        {!isUser && message.recalled && message.recalled.length > 0 && (
          <RecalledTurns recalled={message.recalled} />
        )}

        {!isUser && (errored || message.status === "stopped") && onRetry && (
          <button onClick={onRetry} className="mt-2 text-xs text-foreground underline">
            {t("common.retry")}
          </button>
        )}
      </div>
    </div>
  );
}

/** A delegated sub-agent reply: visually subordinate, labeled with its @handle. */
export function SubagentBubble({
  alias,
  content,
  status,
}: {
  alias: string;
  content: string;
  status: "complete" | "error";
}) {
  const { t } = useTranslation();
  const errored = status === "error";
  return (
    <div className="flex justify-start">
      <div
        className={cn(
          "ml-8 max-w-2xl rounded-md border-l-2 px-3 py-2 text-sm",
          errored ? "border-l-destructive bg-destructive/5" : "border-l-primary/50 bg-muted/40",
        )}
      >
        <div className="mb-1 flex items-center gap-1.5 text-xs text-muted-foreground">
          <span className="rounded bg-muted px-1.5 py-0.5 font-mono">@{alias}</span>
          <span className="uppercase tracking-wide">{t("chat.subagent.tag")}</span>
        </div>
        {errored ? (
          <p className="text-destructive">{t("chat.subagent.failed")}</p>
        ) : (
          <Markdown>{content}</Markdown>
        )}
      </div>
    </div>
  );
}
