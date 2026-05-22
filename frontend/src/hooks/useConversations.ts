import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";

export const conversationsKey = (agentId: string) =>
  ["conversations", agentId] as const;
export const messagesKey = (conversationId: string) =>
  ["messages", conversationId] as const;

export function useConversations(agentId: string | undefined) {
  return useQuery({
    queryKey: agentId ? conversationsKey(agentId) : ["conversations", "none"],
    queryFn: () => api.listConversations(agentId!),
    enabled: !!agentId,
  });
}

export function useMessages(conversationId: string | undefined) {
  return useQuery({
    queryKey: conversationId ? messagesKey(conversationId) : ["messages", "none"],
    queryFn: () => api.listMessages(conversationId!),
    enabled: !!conversationId,
  });
}

export function useCreateConversation(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (title?: string) => api.createConversation(agentId, title),
    onSuccess: () => qc.invalidateQueries({ queryKey: conversationsKey(agentId) }),
  });
}

export function useRenameConversation(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, title }: { id: string; title: string }) =>
      api.renameConversation(id, title),
    onSuccess: () => qc.invalidateQueries({ queryKey: conversationsKey(agentId) }),
  });
}

export function useDeleteConversation(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteConversation(id),
    onSuccess: (_data, id) => {
      qc.invalidateQueries({ queryKey: conversationsKey(agentId) });
      qc.removeQueries({ queryKey: messagesKey(id) });
    },
  });
}
