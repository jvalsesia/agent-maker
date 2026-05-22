import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type ListAgentsParams, type ProviderName } from "@/lib/api";

export const agentsKey = (params: ListAgentsParams = {}) => ["agents", params] as const;
export const agentKey = (id: string) => ["agent", id] as const;

export function useAgents(params: ListAgentsParams = {}) {
  return useQuery({ queryKey: agentsKey(params), queryFn: () => api.listAgents(params) });
}

export function useAgent(id: string | undefined) {
  return useQuery({
    queryKey: id ? agentKey(id) : ["agent", "none"],
    queryFn: () => api.getAgent(id!),
    enabled: !!id,
  });
}

export function useAgentModels(provider: ProviderName | undefined) {
  return useQuery({
    queryKey: ["agent-models", provider ?? "none"],
    queryFn: () => api.getAgentModels(provider!),
    enabled: !!provider,
    staleTime: Infinity,
  });
}

export function useCreateAgent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.createAgent,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["agents"] }),
  });
}

export function useUpdateAgent(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: Parameters<typeof api.updateAgent>[1]) => api.updateAgent(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["agents"] });
      qc.invalidateQueries({ queryKey: agentKey(id) });
    },
  });
}

export function useDeleteAgent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteAgent(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["agents"] }),
  });
}

export function useCloneAgent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name?: string }) => api.cloneAgent(id, name),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["agents"] }),
  });
}

export function useSaveAgentKey(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (key: string) => api.saveAgentKey(id, key),
    onSuccess: () => qc.invalidateQueries({ queryKey: agentKey(id) }),
  });
}

export function useDeleteAgentKey(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => api.deleteAgentKey(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: agentKey(id) }),
  });
}
