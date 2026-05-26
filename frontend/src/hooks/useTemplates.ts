import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { templatesApi, type TemplateCategory } from "@/lib/templates";

export const templatesKey = (category?: TemplateCategory, locale?: string) =>
  ["templates", category ?? "all", locale ?? "en"] as const;

export function useTemplates(category?: TemplateCategory, locale?: string) {
  return useQuery({
    queryKey: templatesKey(category, locale),
    queryFn: () => templatesApi.list(category, locale),
  });
}

export function useAgentTemplate(slug: string | undefined) {
  return useQuery({
    queryKey: ["template-agent", slug ?? "none"],
    queryFn: () => templatesApi.getAgent(slug!),
    enabled: !!slug,
  });
}

export function useSkillTemplate(slug: string | undefined) {
  return useQuery({
    queryKey: ["template-skill", slug ?? "none"],
    queryFn: () => templatesApi.getSkill(slug!),
    enabled: !!slug,
  });
}

export function useAdoptAgent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (slug: string) => templatesApi.adoptAgent(slug),
    onSuccess: (resp) => {
      qc.invalidateQueries({ queryKey: ["agents"] });
      qc.invalidateQueries({ queryKey: ["skills"] });
      qc.invalidateQueries({ queryKey: ["agent-skills", resp.agent.id] });
    },
  });
}

export function useAdoptSkill() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (slug: string) => templatesApi.adoptSkill(slug),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["skills"] }),
  });
}
