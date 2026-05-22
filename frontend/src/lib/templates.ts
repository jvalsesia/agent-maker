import type { Agent, Skill } from "@/lib/api";
import { ApiError, type ApiErrorBody } from "@/lib/api";

export type TemplateCategory =
  | "writing"
  | "research"
  | "productivity"
  | "coding"
  | "learning"
  | "wellbeing";

export const ALL_CATEGORIES: TemplateCategory[] = [
  "writing",
  "research",
  "productivity",
  "coding",
  "learning",
  "wellbeing",
];

export interface AgentTemplateSummary {
  slug: string;
  name: string;
  category: TemplateCategory;
  preamble: string;
  suggested_skills: string[];
}

export interface SkillTemplateSummary {
  slug: string;
  name: string;
  category: TemplateCategory;
  description: string;
}

export interface TemplatesList {
  agents: AgentTemplateSummary[];
  skills: SkillTemplateSummary[];
}

export interface SuggestedSkillPreview {
  slug: string;
  name: string;
  description: string;
}

export interface AgentTemplateDetail {
  slug: string;
  name: string;
  category: TemplateCategory;
  preamble: string;
  system_prompt: string;
  default_provider: string;
  default_model: string;
  suggested_skills: SuggestedSkillPreview[];
}

export interface SkillTemplateDetail {
  slug: string;
  name: string;
  category: TemplateCategory;
  description: string;
  body: string;
}

export interface AdoptAgentResponse {
  agent: Agent;
  attached_skill_ids: string[];
  warnings: { field: string; message: string }[];
}

export interface AdoptSkillResponse {
  skill: Skill;
  warnings: { field: string; message: string }[];
}

async function request<T>(method: string, path: string): Promise<T> {
  const res = await fetch(path, { method });
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new ApiError(res.status, data as ApiErrorBody);
  return data as T;
}

export const templatesApi = {
  list: (category?: TemplateCategory) =>
    request<TemplatesList>(
      "GET",
      `/api/templates${category ? `?category=${category}` : ""}`,
    ),
  getAgent: (slug: string) =>
    request<{ agent: AgentTemplateDetail }>("GET", `/api/templates/agents/${slug}`),
  getSkill: (slug: string) =>
    request<{ skill: SkillTemplateDetail }>("GET", `/api/templates/skills/${slug}`),
  adoptAgent: (slug: string) =>
    request<AdoptAgentResponse>("POST", `/api/templates/agents/${slug}/adopt`),
  adoptSkill: (slug: string) =>
    request<AdoptSkillResponse>("POST", `/api/templates/skills/${slug}/adopt`),
};
