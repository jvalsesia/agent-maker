import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useAgentTemplate, useSkillTemplate } from "@/hooks/useTemplates";

export interface PreviewTarget {
  kind: "agent" | "skill";
  slug: string;
}

interface Props {
  open: boolean;
  target: PreviewTarget | null;
  onClose: () => void;
  onAdopt: (target: PreviewTarget) => void;
  adopting: boolean;
}

export function TemplatePreviewDrawer({ open, target, onClose, onAdopt, adopting }: Props) {
  const { t } = useTranslation();
  const agentQ = useAgentTemplate(target?.kind === "agent" ? target.slug : undefined);
  const skillQ = useSkillTemplate(target?.kind === "skill" ? target.slug : undefined);

  return (
    <Dialog open={open} onOpenChange={(o) => (!o ? onClose() : undefined)}>
      <DialogContent className="max-w-2xl max-h-[85vh] overflow-hidden flex flex-col">
        {target?.kind === "agent" && agentQ.data && (
          <AgentBody detail={agentQ.data.agent} />
        )}
        {target?.kind === "skill" && skillQ.data && (
          <SkillBody detail={skillQ.data.skill} />
        )}
        {(agentQ.isLoading || skillQ.isLoading) && (
          <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
        )}
        <div className="mt-4 flex justify-end gap-2 border-t border-border pt-3">
          <DialogClose asChild>
            <Button variant="ghost">{t("common.close")}</Button>
          </DialogClose>
          <Button
            onClick={() => target && onAdopt(target)}
            disabled={!target || adopting}
          >
            {adopting ? t("templates.drawer.adopting") : t("templates.adopt")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function CategoryBadge({ category }: { category: string }) {
  const { t } = useTranslation();
  return (
    <span className="rounded bg-muted px-2 py-0.5 text-xs uppercase tracking-wide text-muted-foreground">
      {t(`categories.${category}`)}
    </span>
  );
}

function CopyBlock({ value }: { value: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(t);
  }, [copied]);
  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
    } catch {
      toast.error(t("templates.drawer.copyFailed"));
    }
  };
  return (
    <div className="relative">
      <pre className="max-h-72 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/40 p-3 font-mono text-xs">
        {value}
      </pre>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className="absolute right-1 top-1"
        onClick={onCopy}
      >
        <Copy className="h-3.5 w-3.5" />
        <span className="ml-1 text-xs">{copied ? t("templates.drawer.copied") : t("templates.drawer.copy")}</span>
      </Button>
    </div>
  );
}

function AgentBody({ detail }: { detail: import("@/lib/templates").AgentTemplateDetail }) {
  const { t } = useTranslation();
  return (
    <div className="flex-1 overflow-y-auto pr-1">
      <DialogHeader>
        <div className="flex items-center gap-2">
          <DialogTitle>{detail.name}</DialogTitle>
          <CategoryBadge category={detail.category} />
        </div>
        <p className="text-sm text-muted-foreground">{detail.preamble}</p>
      </DialogHeader>
      <div className="mt-4 space-y-3">
        <div>
          <div className="mb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("templates.drawer.systemPrompt")}
          </div>
          <CopyBlock value={detail.system_prompt} />
        </div>
        <div className="text-xs text-muted-foreground">
          {t("templates.drawer.defaultsTo")} <code>{detail.default_provider}</code> ·{" "}
          <code>{detail.default_model}</code>
        </div>
        {detail.suggested_skills.length > 0 && (
          <div>
            <div className="mb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("templates.drawer.suggestedSkills")}
            </div>
            <ul className="space-y-1">
              {detail.suggested_skills.map((s) => (
                <li
                  key={s.slug}
                  className="rounded border border-border bg-card/60 p-2 text-sm"
                >
                  <div className="font-medium">{s.name}</div>
                  <div className="text-xs text-muted-foreground">{s.description}</div>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

function SkillBody({ detail }: { detail: import("@/lib/templates").SkillTemplateDetail }) {
  const { t } = useTranslation();
  return (
    <div className="flex-1 overflow-y-auto pr-1">
      <DialogHeader>
        <div className="flex items-center gap-2">
          <DialogTitle>{detail.name}</DialogTitle>
          <CategoryBadge category={detail.category} />
        </div>
        <p className="text-sm text-muted-foreground">{detail.description}</p>
      </DialogHeader>
      <div className="mt-4">
        <div className="mb-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {t("templates.drawer.body")}
        </div>
        <CopyBlock value={detail.body} />
      </div>
    </div>
  );
}
