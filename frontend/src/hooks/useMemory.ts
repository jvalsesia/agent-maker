import { useMutation, useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";

export function useMemoryStats(conversationId: string | undefined, enabled = true) {
  return useQuery({
    queryKey: ["memory-stats", conversationId ?? "none"],
    queryFn: () => api.memoryStats(conversationId!),
    enabled: !!conversationId && enabled,
  });
}

export function useClearMemory(conversationId: string) {
  return useMutation({
    mutationFn: () => api.clearMemory(conversationId),
  });
}

export function useEmbedPending(conversationId: string) {
  return useMutation({
    mutationFn: () => api.embedPending(conversationId),
  });
}
