import { useState } from "react";
import { Link } from "@tanstack/react-router";
import type { ProjectEvent } from "@/api/types";
import { formatEventTitle, formatEventTypeLabel } from "@/lib/eventFormat";
import { useT } from "@/i18n/context";

const NODE_CLASS: Record<string, string> = {
  error: "error",
  warn: "warn",
  info: "info",
};

export function EventTimeline({
  events,
  compact,
}: {
  events: ProjectEvent[];
  compact?: boolean;
}) {
  const t = useT();
  const [expandedId, setExpandedId] = useState<string | null>(null);

  if (events.length === 0) {
    return <p className="text-sm text-secondary py-4">{t("events.empty")}</p>;
  }

  return (
    <div className="dw-timeline">
      {events.map((e, i) => {
        const open = expandedId === e.id;
        const hasPayload =
          e.payload &&
          typeof e.payload === "object" &&
          Object.keys(e.payload as object).length > 0;
        const expandable = !compact && (Boolean(e.body) || hasPayload);
        const isLast = i === events.length - 1;

        return (
          <div key={e.id} className="dw-timeline-item">
            {!isLast && <div className="dw-timeline-line" />}
            <div className={`dw-timeline-node ${NODE_CLASS[e.severity] ?? "info"}`} />
            <div className="flex flex-col gap-1 min-w-0 flex-1">
              <div className="flex items-start justify-between gap-2">
                <div className="text-sm min-w-0">
                  <Link to="/events/$eventId" params={{ eventId: e.id }} className="font-medium">
                    {formatEventTitle(e, t)}
                  </Link>
                  <span
                    className="text-secondary ml-1 text-xs"
                    title={e.event_type}
                  >
                    · {formatEventTypeLabel(e.event_type, t)}
                  </span>
                </div>
                {expandable && (
                  <button
                    type="button"
                    className="dw-btn-ghost shrink-0"
                    onClick={() => setExpandedId(open ? null : e.id)}
                  >
                    {open ? t("events.collapse") : t("events.expand")}
                  </button>
                )}
              </div>
              {!open && e.body && (
                <p className="text-xs text-secondary line-clamp-2">{e.body}</p>
              )}
              {open && (
                <div className="bg-surface-container-low rounded p-3 text-xs font-code overflow-x-auto">
                  {e.body && <pre className="whitespace-pre-wrap m-0">{e.body}</pre>}
                  {hasPayload && (
                    <pre className="whitespace-pre-wrap m-0 mt-2">
                      {JSON.stringify(e.payload, null, 2)}
                    </pre>
                  )}
                </div>
              )}
              <time className="text-xs text-secondary">{e.occurred_at}</time>
            </div>
          </div>
        );
      })}
    </div>
  );
}
