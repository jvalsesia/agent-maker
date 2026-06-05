import {
  forwardRef,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { Send, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { useDrafts } from "@/hooks/useDrafts";

interface Props {
  conversationId: string;
  streaming: boolean;
  /** Attached sub-agent handles (F11) powering `@`-autocomplete. */
  aliases?: string[];
  onSend: (content: string) => void;
  onStop: () => void;
}

/** Imperative API exposed to siblings (e.g. the chat sub-agents bar) so they can
 *  drive caret-accurate `@handle` insertion using the composer's own textarea. */
export interface ComposerHandle {
  /** Insert `@alias ` at the caret (or end of draft if unfocused) and refocus.
   *  No-op for the splice when the exact `@alias` token is already present. */
  insertMention(alias: string): void;
}

/** Word-bounded position just after an existing `@alias` token in `text`, or -1. */
function existingMentionEnd(text: string, alias: string): number {
  const token = `@${alias}`;
  let from = 0;
  for (;;) {
    const at = text.indexOf(token, from);
    if (at < 0) return -1;
    const before = at === 0 || /\s/.test(text[at - 1]);
    const nextCh = text[at + token.length];
    const after = nextCh === undefined || !/[a-zA-Z0-9-]/.test(nextCh);
    if (before && after) return at + token.length;
    from = at + token.length;
  }
}

interface Mention {
  /** Index of the `@` in the draft. */
  start: number;
  /** The partial handle typed after `@`. */
  query: string;
  /** Caret position when the mention context was computed. */
  caret: number;
}

/** Find the `@handle` being typed at the caret, if any. A mention starts at the
 *  beginning of the text or after whitespace, with only `[a-z0-9-]` after `@`. */
function mentionAt(text: string, caret: number): Mention | null {
  let i = caret - 1;
  while (i >= 0) {
    const ch = text[i];
    if (ch === "@") {
      const boundary = i === 0 || /\s/.test(text[i - 1]);
      return boundary ? { start: i, query: text.slice(i + 1, caret), caret } : null;
    }
    if (!/[a-zA-Z0-9-]/.test(ch)) return null;
    i--;
  }
  return null;
}

const MAX_SUGGESTIONS = 6;

/**
 * Chat input: Enter sends, Shift+Enter inserts a newline, Cmd/Ctrl+K focuses.
 * Typing `@` surfaces an autocomplete of the agent's attached sub-agent handles;
 * Up/Down navigate, Enter/Tab accept, Esc dismisses. While a reply streams, the
 * send button becomes a Stop button. The draft is preserved per conversation.
 */
export const Composer = forwardRef<ComposerHandle, Props>(function Composer(
  { conversationId, streaming, aliases = [], onSend, onStop },
  handleRef,
) {
  const { t } = useTranslation();
  const { getDraft, setDraft } = useDrafts();
  const taRef = useRef<HTMLTextAreaElement>(null);
  const draft = getDraft(conversationId);

  const [mention, setMention] = useState<Mention | null>(null);
  const [active, setActive] = useState(0);

  /** Place the caret at `pos` in the textarea on the next frame, focusing it. */
  function focusCaret(pos: number) {
    requestAnimationFrame(() => {
      const el = taRef.current;
      if (el) {
        el.focus();
        el.setSelectionRange(pos, pos);
      }
    });
  }

  /** Insert (or reuse) an `@alias` mention at the caret. Reused by the chat
   *  sub-agents bar via the imperative handle and shares `accept()`'s mechanics. */
  function insertMention(alias: string) {
    const current = getDraft(conversationId);
    const dedupeEnd = existingMentionEnd(current, alias);
    if (dedupeEnd >= 0) {
      focusCaret(dedupeEnd);
      return;
    }
    const caret = taRef.current?.selectionStart ?? current.length;
    const before = current.slice(0, caret);
    const after = current.slice(caret);
    const next = `${before}@${alias} ${after}`;
    setDraft(conversationId, next);
    setMention(null);
    focusCaret(caret + alias.length + 2);
  }

  useImperativeHandle(handleRef, () => ({ insertMention }), [conversationId]);

  const suggestions = useMemo(() => {
    if (!mention) return [];
    const q = mention.query.toLowerCase();
    return aliases.filter((a) => a.toLowerCase().startsWith(q)).slice(0, MAX_SUGGESTIONS);
  }, [mention, aliases]);

  const menuOpen = suggestions.length > 0;

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        taRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  function refresh(value: string, caret: number) {
    const m = aliases.length ? mentionAt(value, caret) : null;
    setMention(m);
    setActive(0);
  }

  function onChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const value = e.target.value;
    setDraft(conversationId, value);
    const caret = e.target.selectionStart || value.length;
    refresh(value, caret);
  }

  function accept(alias: string) {
    if (!mention) return;
    const before = draft.slice(0, mention.start);
    const after = draft.slice(mention.caret);
    const next = `${before}@${alias} ${after}`;
    setDraft(conversationId, next);
    setMention(null);
    focusCaret(mention.start + alias.length + 2);
  }

  function submit() {
    const text = draft.trim();
    if (!text || streaming) return;
    onSend(text);
    setDraft(conversationId, "");
    setMention(null);
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (menuOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActive((a) => (a + 1) % suggestions.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setActive((a) => (a - 1 + suggestions.length) % suggestions.length);
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        accept(suggestions[active]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setMention(null);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  return (
    <div className="border-t border-border p-3">
      {streaming && (
        <p className="mb-2 text-xs text-muted-foreground" role="status">
          {t("chat.composer.generating")}
        </p>
      )}
      <div className="relative flex items-end gap-2">
        {menuOpen && (
          <ul
            role="listbox"
            aria-label={t("chat.composer.mentionAria")}
            className="absolute bottom-full left-0 z-10 mb-1 w-64 overflow-hidden rounded-md border border-border bg-popover shadow-md"
          >
            {suggestions.map((alias, i) => (
              <li key={alias}>
                <button
                  type="button"
                  role="option"
                  aria-selected={i === active}
                  className={`flex w-full items-center px-3 py-1.5 text-left text-sm ${
                    i === active ? "bg-accent text-accent-foreground" : ""
                  }`}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    accept(alias);
                  }}
                >
                  <span className="font-mono">@{alias}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
        <Textarea
          ref={taRef}
          aria-label={t("chat.composer.ariaMessage")}
          placeholder={t("chat.composer.placeholder")}
          value={draft}
          onChange={onChange}
          onKeyDown={onKeyDown}
          rows={3}
          className="resize-none"
        />
        {streaming ? (
          <Button variant="destructive" size="icon" aria-label={t("chat.composer.ariaStop")} onClick={onStop}>
            <Square className="h-4 w-4" />
          </Button>
        ) : (
          <Button
            size="icon"
            aria-label={t("chat.composer.ariaSend")}
            disabled={!draft.trim()}
            onClick={submit}
          >
            <Send className="h-4 w-4" />
          </Button>
        )}
      </div>
    </div>
  );
});
