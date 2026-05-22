import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type ListSkillsParams } from "@/lib/api";

export const skillsKey = (params: ListSkillsParams = {}) => ["skills", params] as const;
export const skillKey = (id: string) => ["skill", id] as const;

export function useSkills(params: ListSkillsParams = {}) {
  return useQuery({ queryKey: skillsKey(params), queryFn: () => api.listSkills(params) });
}

export function useSkill(id: string | undefined) {
  return useQuery({
    queryKey: id ? skillKey(id) : ["skill", "none"],
    queryFn: () => api.getSkill(id!),
    enabled: !!id,
  });
}

export function useCreateSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: api.createSkill,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["skills"] }),
  });
}

export function useUpdateSkill(id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: Parameters<typeof api.updateSkill>[1]) => api.updateSkill(id, body),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["skills"] });
      qc.invalidateQueries({ queryKey: skillKey(id) });
    },
  });
}

export function useDeleteSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteSkill(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["skills"] });
      qc.invalidateQueries({ queryKey: ["agents"] });
    },
  });
}

export function useCloneSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name?: string }) => api.cloneSkill(id, name),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["skills"] }),
  });
}
