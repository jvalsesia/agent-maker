import { useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { BookOpen } from "lucide-react";
import { useLocale } from "@/hooks/useLocale";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/cn";
import {
  ALL_CATEGORIES,
  type AgentTemplateSummary,
  type SkillTemplateSummary,
  type TemplateCategory,
} from "@/lib/templates";
import { ApiError } from "@/lib/api";
import {
  useAdoptAgent,
  useAdoptSkill,
  useTemplates,
} from "@/hooks/useTemplates";
import {
  TemplatePreviewDrawer,
  type PreviewTarget,
} from "./TemplatePreviewDrawer";

type Kind = "all" | "agent" | "skill";

export function TemplatesGallery() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const locale = useLocale();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<TemplateCategory | "all">("all");
  const [kind, setKind] = useState<Kind>("all");
  const [preview, setPreview] = useState<PreviewTarget | null>(null);

  const { data, isLoading } = useTemplates(undefined, locale);
  const adoptAgent = useAdoptAgent();
  const adoptSkill = useAdoptSkill();
  const adopting = adoptAgent.isPending || adoptSkill.isPending;

  const { agents, skills } = useMemo(() => {
    const s = search.trim().toLowerCase();
    const matchCat = (c: TemplateCategory) => category === "all" || category === c;
    const all = data ?? { agents: [], skills: [] };
    const agentsF = all.agents.filter(
      (a) =>
        matchCat(a.category) &&
        (kind === "all" || kind === "agent") &&
        (!s ||
          a.name.toLowerCase().includes(s) ||
          a.preamble.toLowerCase().includes(s)),
    );
    const skillsF = all.skills.filter(
      (sk) =>
        matchCat(sk.category) &&
        (kind === "all" || kind === "skill") &&
        (!s ||
          sk.name.toLowerCase().includes(s) ||
          sk.description.toLowerCase().includes(s)),
    );
    return { agents: agentsF, skills: skillsF };
  }, [data, search, category, kind]);

  const clearFilters = () => {
    setSearch("");
    setCategory("all");
    setKind("all");
  };

  const onAdopt = async (target: PreviewTarget) => {
    try {
      if (target.kind === "agent") {
        const r = await adoptAgent.mutateAsync(target.slug);
        toast.success(t("templates.adoptedAgent", { name: r.agent.name }), {
          action: {
            label: t("templates.openAgent"),
            onClick: () => navigate(`/agents/${r.agent.id}`),
          },
        });
      } else {
        const r = await adoptSkill.mutateAsync(target.slug);
        toast.success(t("templates.adoptedSkill", { name: r.skill.name }), {
          action: {
            label: t("templates.openSkill"),
            onClick: () => navigate(`/skills/${r.skill.id}`),
          },
        });
      }
      setPreview(null);
    } catch (e) {
      const msg =
        e instanceof ApiError ? e.body.error.message : t("templates.adoptError");
      toast.error(msg);
    }
  };

  const total = agents.length + skills.length;

  return (
    <div className="mx-auto max-w-5xl">
      <div className="sticky top-0 z-10 -mx-6 bg-background/95 px-6 pb-4 pt-1 backdrop-blur">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight">{t("templates.title")}</h1>
            <p className="text-sm text-muted-foreground mt-1">
              {t("templates.subtitle")}
            </p>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-3">
          <Input
            placeholder={t("templates.searchPlaceholder")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="max-w-sm"
          />
          <div className="ml-auto flex rounded-md border border-border p-0.5 text-xs">
            {(["all", "agent", "skill"] as const).map((k) => (
              <button
                key={k}
                onClick={() => setKind(k)}
                className={cn(
                  "rounded px-3 py-1 capitalize",
                  kind === k
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {k === "all" ? t("templates.kindAll") : k === "agent" ? t("templates.kindAgents") : t("templates.kindSkills")}
              </button>
            ))}
          </div>
        </div>

        <div className="mt-3 flex flex-wrap gap-1.5 text-xs">
          <CategoryChip
            label={t("categories.all")}
            active={category === "all"}
            onClick={() => setCategory("all")}
          />
          {ALL_CATEGORIES.map((c) => (
            <CategoryChip
              key={c}
              label={t(`categories.${c}`)}
              active={category === c}
              onClick={() => setCategory(c)}
            />
          ))}
        </div>
      </div>

      {isLoading ? (
        <p className="mt-6 text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : total === 0 ? (
        <div className="mt-10 rounded-md border border-dashed border-border p-10 text-center">
          <BookOpen className="mx-auto mb-3 h-8 w-8 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">
            {t("templates.emptyMatch")}
          </p>
          <button
            onClick={clearFilters}
            className="mt-3 text-sm text-foreground underline"
          >
            {t("templates.clearFilters")}
          </button>
        </div>
      ) : (
        <div className="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-2">
          {agents.map((a) => (
            <AgentCard
              key={a.slug}
              agent={a}
              onPreview={() => setPreview({ kind: "agent", slug: a.slug })}
              onAdopt={() => onAdopt({ kind: "agent", slug: a.slug })}
            />
          ))}
          {skills.map((s) => (
            <SkillCard
              key={s.slug}
              skill={s}
              onPreview={() => setPreview({ kind: "skill", slug: s.slug })}
              onAdopt={() => onAdopt({ kind: "skill", slug: s.slug })}
            />
          ))}
        </div>
      )}

      <TemplatePreviewDrawer
        open={!!preview}
        target={preview}
        onClose={() => setPreview(null)}
        onAdopt={onAdopt}
        adopting={adopting}
      />
    </div>
  );
}

function CategoryChip({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-full border px-3 py-1 capitalize transition-colors",
        active
          ? "border-foreground bg-foreground text-background"
          : "border-border bg-card/60 text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
    </button>
  );
}

function CardShell({
  kindLabel,
  category,
  title,
  description,
  isFallback,
  onPreview,
  onAdopt,
}: {
  kindLabel: string;
  category: string;
  title: string;
  description: string;
  isFallback?: boolean;
  onPreview: () => void;
  onAdopt: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col rounded-md border border-border bg-card/60 p-4">
      <div className="mb-1 flex items-center gap-2 text-xs">
        <span className="rounded bg-muted px-1.5 py-0.5 uppercase tracking-wide text-muted-foreground">
          {kindLabel}
        </span>
        <span className="rounded bg-muted px-1.5 py-0.5 uppercase tracking-wide text-muted-foreground">
          {t(`categories.${category}`)}
        </span>
        {isFallback && (
          <span
            className="rounded bg-muted px-1.5 py-0.5 uppercase tracking-wide text-muted-foreground"
            title={t("templates.fallbackTooltip")}
          >
            {t("templates.fallbackBadge")}
          </span>
        )}
      </div>
      <div className="font-medium">{title}</div>
      <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{description}</p>
      <div className="mt-3 flex gap-2">
        <Button variant="ghost" size="sm" onClick={onPreview}>
          {t("templates.preview")}
        </Button>
        <Button size="sm" onClick={onAdopt}>
          {t("templates.adopt")}
        </Button>
      </div>
    </div>
  );
}

function AgentCard({
  agent,
  onPreview,
  onAdopt,
}: {
  agent: AgentTemplateSummary;
  onPreview: () => void;
  onAdopt: () => void;
}) {
  const { t } = useTranslation();
  return (
    <CardShell
      kindLabel={t("templates.agentKind")}
      category={agent.category}
      title={agent.name}
      description={agent.preamble}
      isFallback={agent.is_fallback}
      onPreview={onPreview}
      onAdopt={onAdopt}
    />
  );
}

function SkillCard({
  skill,
  onPreview,
  onAdopt,
}: {
  skill: SkillTemplateSummary;
  onPreview: () => void;
  onAdopt: () => void;
}) {
  const { t } = useTranslation();
  return (
    <CardShell
      kindLabel={t("templates.skillKind")}
      category={skill.category}
      title={skill.name}
      description={skill.description}
      isFallback={skill.is_fallback}
      onPreview={onPreview}
      onAdopt={onAdopt}
    />
  );
}
