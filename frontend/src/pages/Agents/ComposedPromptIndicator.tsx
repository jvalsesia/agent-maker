import { AlertTriangle } from "lucide-react";
import { useComposePreview } from "@/hooks/useAgentSkills";
import { useLocale } from "@/hooks/useLocale";
import { formatNumber } from "@/lib/format";

export function ComposedPromptIndicator({ agentId }: { agentId: string }) {
  const locale = useLocale();
  const { data, isLoading, isError } = useComposePreview(agentId);

  if (isLoading) {
    return (
      <p className="text-xs text-muted-foreground">Composed prompt: calculating…</p>
    );
  }
  if (isError || !data) {
    return <p className="text-xs text-muted-foreground">Composed prompt: —</p>;
  }

  const pct = Math.round(data.fraction * 100);
  const summary = `Composed prompt: ${formatNumber(data.length_chars, locale)} / ${formatNumber(
    data.model_context_chars,
    locale,
  )} chars (${pct}%)`;

  if (data.fraction >= 0.95) {
    return (
      <div className="flex items-start gap-2 rounded-md border border-destructive/50 bg-destructive/5 px-3 py-2 text-xs text-destructive">
        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
        <div>
          <div className="font-medium">{summary}</div>
          <div>near model context limit — chat may fail</div>
        </div>
      </div>
    );
  }
  if (data.fraction >= 0.8) {
    return (
      <div className="flex items-start gap-2 rounded-md border border-amber-500/50 bg-amber-500/5 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
        <div>
          <div className="font-medium">{summary}</div>
          <div>{data.warning ?? "consider shortening or detaching skills"}</div>
        </div>
      </div>
    );
  }
  return <p className="text-xs text-muted-foreground">{summary}</p>;
}
