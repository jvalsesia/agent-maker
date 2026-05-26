import { useTranslation } from "react-i18next";
import { cn } from "@/lib/cn";
import type { Message } from "@/lib/api";
import { Markdown } from "./Markdown";
import { RecalledTurns } from "./RecalledTurns";

interface Props {
  message: Message;
  /** Retry an errored/stopped assistant turn. */
  onRetry?: () => void;
}

/** A persisted chat message: markdown for assistant turns, plain text for the
 *  user, plus a model/token chip, status indicators, retry, and recalled turns. */
export function MessageBubble({ message, onRetry }: Props) {
  const { t } = useTranslation();
  const isUser = message.role === "user";
  const errored = message.status === "error";

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
