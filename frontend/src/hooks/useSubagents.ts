import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type AttachSubagentInput, type UpdateSubagentInput } from "@/lib/api";

export const attachedSubagentsKey = (agentId: string) =>
  ["agent-subagents", agentId] as const;

export function useAttachedSubagents(agentId: string | undefined) {
  return useQuery({
    queryKey: agentId ? attachedSubagentsKey(agentId) : ["agent-subagents", "none"],
    queryFn: () => api.listSubagents(agentId!),
    enabled: !!agentId,
  });
}

export function useAttachSubagent(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: AttachSubagentInput) => api.attachSubagent(agentId, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: attachedSubagentsKey(agentId) });
    },
  });
}

export function useUpdateSubagent(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ childId, input }: { childId: string; input: UpdateSubagentInput }) =>
      api.updateSubagent(agentId, childId, input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: attachedSubagentsKey(agentId) });
    },
  });
}

export function useReorderSubagents(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (orderedChildIds: string[]) =>
      api.reorderSubagents(agentId, orderedChildIds),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: attachedSubagentsKey(agentId) });
    },
  });
}

export function useDetachSubagent(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (childId: string) => api.detachSubagent(agentId, childId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: attachedSubagentsKey(agentId) });
    },
  });
}
