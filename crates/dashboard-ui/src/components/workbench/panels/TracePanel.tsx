import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { ToolDetailPanel } from "@/components/TranscriptToolBlock";
import { useT } from "@/i18n/context";

type Props = {
  sessionId: string;
  live?: boolean;
  isRunning?: boolean;
  selectedTool?: TranscriptBlock | null;
  onSelectTool?: (tool: TranscriptBlock | null) => void;
};

export function TracePanel({
  sessionId,
  live,
  isRunning = false,
  selectedTool,
}: Props) {
  const t = useT();
  const running = Boolean(isRunning);

  const trace = useQuery({
    queryKey: ["session-trace-inspector", sessionId],
    queryFn: () => api.sessionTrace(sessionId),
    enabled: Boolean(sessionId),
    staleTime: running ? 3_000 : 15_000,
    refetchInterval: running && !live ? 6_000 : false,
    placeholderData: (prev) => prev,
  });

  const traceEvents = (trace.data?.trace.events ?? []).filter((evt) =>
    evt.event_type.startsWith("tool_call"),
  );
  const recentTrace = traceEvents.slice(-12).reverse();

  return (
    <div className="flex flex-col min-h-0 h-full overflow-y-auto">
      <section className="border-b border-outline-variant/60">
        <h3 className="px-3 py-2 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 flex items-center gap-1.5 bg-surface-container-low/50">
          <Icon name="timeline" size={14} />
          {t("conversations.inspectorTimeline")}
          {recentTrace.length > 0 && (
            <span className="text-outline normal-case">({recentTrace.length})</span>
          )}
        </h3>
        {recentTrace.length === 0 ? (
          <p className="text-xs text-secondary m-0 px-3 py-2">
            {t("conversations.inspectorTimelineEmpty")}
          </p>
        ) : (
          <ul className="m-0 p-0 list-none">
            {recentTrace.map((evt, index) => (
              <li
                key={`${evt.event_type}-${evt.occurred_at}-${index}`}
                className="px-3 py-1.5 text-xs border-b border-outline-variant/30 last:border-0"
              >
                <span className="font-medium text-on-surface block truncate">
                  {(evt.payload?.name as string | undefined) ?? evt.title}
                </span>
                <span className="text-secondary font-code truncate block">
                  {evt.event_type.replace("tool_call_", "")}
                  {typeof evt.payload?.command === "string" ? ` · ${evt.payload.command}` : ""}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="flex-1 min-h-0">
        <h3 className="px-3 py-2 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 flex items-center gap-1.5 bg-surface-container-low/50">
          <Icon name="build" size={14} />
          {t("conversations.inspectorDetail")}
        </h3>
        <ToolDetailPanel tool={selectedTool ?? null} />
      </section>

      <div className="px-3 pt-2 pb-4 border-t border-outline-variant/60 mt-auto shrink-0">
        <Link
          to="/sessions/$sessionId"
          params={{ sessionId }}
          className="text-xs text-secondary no-underline hover:text-primary inline-flex items-center gap-1"
        >
          <Icon name="timeline" size={14} />
          {t("conversations.openDetail")}
        </Link>
      </div>
    </div>
  );
}
