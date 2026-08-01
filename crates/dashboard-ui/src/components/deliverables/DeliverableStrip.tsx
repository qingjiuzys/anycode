import { useMemo, useState } from "react";
import {
  DeliverableCard,
  type DeliverableCardProps,
} from "@/components/deliverables/DeliverableCard";
import { useT } from "@/i18n/context";
import { isProcessArtifactPath } from "@/lib/deliverablePath";

const COLLAPSED_VISIBLE = 3;

type Props = {
  items: DeliverableCardProps[];
  projectId?: string;
};

function dedupeItems(items: DeliverableCardProps[]): DeliverableCardProps[] {
  const seen = new Set<string>();
  const out: DeliverableCardProps[] = [];
  for (const item of items) {
    if (!item.path?.trim()) continue;
    if (isProcessArtifactPath(item.path)) continue;
    const key = item.path.split(/[/\\]/).pop()?.toLowerCase() ?? item.path.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(item);
  }
  return out;
}

/** Horizontal deliverable cards with a collapsed "…" overflow. */
export function DeliverableStrip({ items, projectId }: Props) {
  const t = useT();
  const [expanded, setExpanded] = useState(false);
  const cleaned = useMemo(() => dedupeItems(items), [items]);
  if (cleaned.length === 0) return null;

  const visible = expanded ? cleaned : cleaned.slice(0, COLLAPSED_VISIBLE);
  const hiddenCount = cleaned.length - visible.length;

  return (
    <div className="deliverable-strip">
      <div className="deliverable-strip__row">
        {visible.map((item) => (
          <div key={item.path} className="deliverable-strip__item">
            <DeliverableCard
              {...item}
              projectId={item.projectId ?? projectId}
            />
          </div>
        ))}
        {hiddenCount > 0 ? (
          <button
            type="button"
            className="deliverable-strip__more"
            onClick={() => setExpanded(true)}
            aria-expanded={false}
          >
            <span className="deliverable-strip__more-dots" aria-hidden>
              …
            </span>
            <span className="deliverable-strip__more-label">
              {t("common.showMore")} ({hiddenCount})
            </span>
          </button>
        ) : null}
        {expanded && cleaned.length > COLLAPSED_VISIBLE ? (
          <button
            type="button"
            className="deliverable-strip__more deliverable-strip__more--collapse"
            onClick={() => setExpanded(false)}
            aria-expanded
          >
            {t("common.collapse")}
          </button>
        ) : null}
      </div>
    </div>
  );
}
