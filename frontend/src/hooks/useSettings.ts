import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";

export const settingsKey = ["settings"] as const;

export function useSettings() {
  return useQuery({ queryKey: settingsKey, queryFn: api.getSettings });
}

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.putSettings,
    onSuccess: (data) => qc.setQueryData(settingsKey, data),
  });
}

export function useHealth() {
  return useQuery({
    queryKey: ["health"],
    queryFn: api.health,
    retry: 1,
    refetchInterval: 30_000,
  });
}
