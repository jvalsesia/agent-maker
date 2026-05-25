import { useCallback, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ApiError, chatStream, type ChatRecalledTurn, type ChatStartInput } from "@/lib/api";
import { conversationsKey, messagesKey } from "./useConversations";

/** Live state of the assistant reply currently being streamed. */
export interface DraftAssistant {
  content: string;
  recalled: ChatRecalledTurn[];
  degraded: boolean;
  model: string | null;
}

export interface ChatError {
  message: string;
  code?: string;
}

const EMPTY_DRAFT: DraftAssistant = { content: "", recalled: [], degraded: false, model: null };

/**
 * Drives one in-flight chat turn per conversation: sends a message (or retries
 * the last one), accumulates streamed chunks into a provisional assistant
 * bubble, and exposes a `stop()` that aborts the stream. When the turn settles,
 * the persisted messages are refetched so the canonical rows (with recalled
 * turns) replace the provisional state.
 */
export function useChat(conversationId: string | undefined, agentId: string) {
  const qc = useQueryClient();
  const [streaming, setStreaming] = useState(false);
  const [assistant, setAssistant] = useState<DraftAssistant | null>(null);
  const [pendingUser, setPendingUser] = useState<string | null>(null);
  const [error, setError] = useState<ChatError | null>(null);
  const ctrl = useRef<AbortController | null>(null);

  const run = useCallback(
    async (input: ChatStartInput, optimisticUser: string | null) => {
      if (!conversationId) return;
      setError(null);
      setStreaming(true);
      setPendingUser(optimisticUser);
      setAssistant({ ...EMPTY_DRAFT });

      const controller = new AbortController();
      ctrl.current = controller;
      let aborted = false;

      try {
        for await (const ev of chatStream(conversationId, input, controller.signal)) {
          if (ev.type === "meta") {
            setAssistant((a) => ({
              ...(a ?? EMPTY_DRAFT),
              recalled: ev.recalled,
              degraded: ev.degraded,
              model: ev.model,
            }));
          } else if (ev.type === "chunk") {
            setAssistant((a) => ({ ...(a ?? EMPTY_DRAFT), content: (a?.content ?? "") + ev.content }));
          } else if (ev.type === "error") {
            setError({ message: ev.message, code: ev.code });
          }
          // "done" needs no extra handling — the finally block refetches.
        }
      } catch (e) {
        if (controller.signal.aborted) {
          aborted = true; // stopped by the user; backend persists the partial
        } else if (e instanceof ApiError) {
          setError({ message: e.message, code: e.body.error?.code });
        } else {
          setError({ message: e instanceof Error ? e.message : String(e) });
        }
      } finally {
        setStreaming(false);
        ctrl.current = null;
        // On a user-initiated stop, give the backend a moment to persist the
        // partial reply as "stopped" before we refetch the canonical rows.
        if (aborted) await new Promise((r) => setTimeout(r, 250));
        await qc.invalidateQueries({ queryKey: messagesKey(conversationId) });
        qc.invalidateQueries({ queryKey: conversationsKey(agentId) });
        setAssistant(null);
        setPendingUser(null);
      }
    },
    [conversationId, agentId, qc],
  );

  const send = useCallback((content: string) => run({ content }, content), [run]);
  const retry = useCallback(() => run({ retry: true }, null), [run]);
  const stop = useCallback(() => ctrl.current?.abort(), []);

  return { streaming, assistant, pendingUser, error, send, retry, stop };
}
