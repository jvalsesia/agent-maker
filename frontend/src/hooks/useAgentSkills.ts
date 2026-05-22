import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api";

export const attachedSkillsKey = (agentId: string) =>
  ["agent-skills", agentId] as const;
export const composeKey = (agentId: string) => ["agent-compose", agentId] as const;

export function useAttachedSkills(agentId: string | undefined) {
  return useQuery({
    queryKey: agentId ? attachedSkillsKey(agentId) : ["agent-skills", "none"],
    queryFn: () => api.listAttachedSkills(agentId!),
    enabled: !!agentId,
  });
}

export function useComposePreview(agentId: string | undefined) {
  return useQuery({
    queryKey: agentId ? composeKey(agentId) : ["agent-compose", "none"],
    queryFn: () => api.getComposePreview(agentId!),
    enabled: !!agentId,
  });
}

export function useReplaceAttachedSkills(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (skill_ids: string[]) => api.replaceAttachedSkills(agentId, skill_ids),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: attachedSkillsKey(agentId) });
      qc.invalidateQueries({ queryKey: composeKey(agentId) });
      qc.invalidateQueries({ queryKey: ["skills"] });
    },
  });
}

export function useDetachSkill(agentId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (skillId: string) => api.detachSkill(agentId, skillId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: attachedSkillsKey(agentId) });
      qc.invalidateQueries({ queryKey: composeKey(agentId) });
      qc.invalidateQueries({ queryKey: ["skills"] });
    },
  });
}
