import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import { useConversations, useMessages } from "@/hooks/useConversations";
import { DraftsProvider, useDrafts } from "@/hooks/useDrafts";
import { ConversationSidebar } from "./ConversationSidebar";
import { MessageList } from "./MessageList";

export function ChatPage() {
  const { id: agentId } = useParams<{ id: string }>();
  if (!agentId) return null;
  return (
    <DraftsProvider>
      <ChatInner agentId={agentId} />
    </DraftsProvider>
  );
}

function ChatInner({ agentId }: { agentId: string }) {
  const { data, isLoading } = useConversations(agentId);
  const conversations = data?.conversations ?? [];
  const [activeId, setActiveId] = useState<string | null>(null);

  // Auto-select the first conversation when the list loads (or after the
  // active one is deleted and removed from the list).
  useEffect(() => {
    if (conversations.length === 0) {
      setActiveId(null);
      return;
    }
    if (!activeId || !conversations.some((c) => c.id === activeId)) {
      setActiveId(conversations[0].id);
    }
  }, [conversations, activeId]);

  return (
    <div className="-m-6 flex h-[calc(100vh-0px)] min-h-0">
      <ConversationSidebar
        agentId={agentId}
        conversations={conversations}
        activeId={activeId}
        onSelect={setActiveId}
      />
      <main className="flex min-w-0 flex-1 flex-col">
        <Header agentId={agentId} />
        <ChatSurface activeId={activeId} loadingList={isLoading} />
      </main>
    </div>
  );
}

function Header({ agentId }: { agentId: string }) {
  return (
    <div className="flex items-center gap-2 border-b border-border px-4 py-2 text-sm">
      <Link
        to={`/agents/${agentId}`}
        className="inline-flex items-center gap-1 text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="h-4 w-4" /> Edit agent
      </Link>
    </div>
  );
}

function ChatSurface({ activeId, loadingList }: { activeId: string | null; loadingList: boolean }) {
  const { data, isLoading, error, refetch } = useMessages(activeId ?? undefined);
  const { getDraft, setDraft } = useDrafts();

  if (loadingList) {
    return <p className="p-6 text-sm text-muted-foreground">Loading…</p>;
  }
  if (!activeId) {
    return (
      <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
        Select or create a conversation to begin.
      </div>
    );
  }

  return (
    <>
      <MessageList
        messages={data?.messages ?? []}
        loading={isLoading}
        error={error}
        onRetry={() => refetch()}
      />
      <div className="border-t border-border p-3">
        <Textarea
          placeholder="Composer arrives with F07 — drafts are preserved across conversation switches."
          value={getDraft(activeId)}
          onChange={(e) => setDraft(activeId, e.target.value)}
          rows={3}
          className="resize-none"
        />
      </div>
    </>
  );
}
