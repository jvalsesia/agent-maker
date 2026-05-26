import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import type { RecalledRef } from "@/lib/api";
import { useLocale } from "@/hooks/useLocale";
import { formatDateTime, formatPercent } from "@/lib/format";

/**
 * Collapsible "Recalled N earlier turns" indicator shown under an assistant
 * message, listing each retrieved turn with its timestamp and similarity.
 */
export function RecalledTurns({ recalled }: { recalled: RecalledRef[] }) {
  const [open, setOpen] = useState(false);
  const locale = useLocale();
  if (recalled.length === 0) return null;

  return (
    <div className="mt-2 border-t border-border/60 pt-2">
      <button
        onClick={() => setOpen((o) => !o)}
        className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        aria-expanded={open}
      >
        {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        Recalled {recalled.length} earlier turn{recalled.length === 1 ? "" : "s"}
      </button>
      {open && (
        <ul className="mt-2 space-y-2">
          {recalled.map((r) => (
            <li key={r.message_id} className="rounded border border-border/60 bg-background/50 p-2 text-xs">
              <div className="mb-1 flex items-center justify-between text-muted-foreground">
                <span className="uppercase tracking-wide">{r.role}</span>
                <span>
                  {formatDateTime(r.created_at, locale)} · {formatPercent(r.similarity, locale)} match
                </span>
              </div>
              <p className="whitespace-pre-wrap text-foreground/80 line-clamp-4">{r.content}</p>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
