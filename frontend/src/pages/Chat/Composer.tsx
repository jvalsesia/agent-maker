import { useEffect, useRef, type KeyboardEvent } from "react";
import { Send, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useDrafts } from "@/hooks/useDrafts";

interface Props {
  conversationId: string;
  streaming: boolean;
  onSend: (content: string) => void;
  onStop: () => void;
}

/**
 * Chat input: Enter sends, Shift+Enter inserts a newline, Cmd/Ctrl+K focuses.
 * While a reply streams, the send button becomes a Stop button and sending is
 * blocked with an inline notice. The draft is preserved per conversation.
 */
export function Composer({ conversationId, streaming, onSend, onStop }: Props) {
  const { getDraft, setDraft } = useDrafts();
  const ref = useRef<HTMLTextAreaElement>(null);
  const draft = getDraft(conversationId);

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        ref.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  function submit() {
    const text = draft.trim();
    if (!text || streaming) return;
    onSend(text);
    setDraft(conversationId, "");
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  return (
    <div className="border-t border-border p-3">
      {streaming && (
        <p className="mb-2 text-xs text-muted-foreground" role="status">
          Generating a response… new messages are blocked until it finishes.
        </p>
      )}
      <div className="flex items-end gap-2">
        <Textarea
          ref={ref}
          aria-label="Message"
          placeholder="Send a message…  (Enter to send, Shift+Enter for newline)"
          value={draft}
          onChange={(e) => setDraft(conversationId, e.target.value)}
          onKeyDown={onKeyDown}
          rows={3}
          className="resize-none"
        />
        {streaming ? (
          <Button variant="destructive" size="icon" aria-label="Stop" onClick={onStop}>
            <Square className="h-4 w-4" />
          </Button>
        ) : (
          <Button
            size="icon"
            aria-label="Send"
            disabled={!draft.trim()}
            onClick={submit}
          >
            <Send className="h-4 w-4" />
          </Button>
        )}
      </div>
    </div>
  );
}
