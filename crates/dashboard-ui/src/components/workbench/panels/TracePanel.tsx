import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { ToolDetailPanel } from "@/components/TranscriptToolBlock";
import { ExecutionTraceGraph } from "@/components/workbench/ExecutionTraceGraph";
import { useT } from "@/i18n/context";
import { resolveCanonicalTranscriptBlocks } from "@/lib/liveTranscript";
import type { ChatStreamEvent } from "@/lib/liveTranscript";
import { sessionDetailSearch } from "@/lib/sessionLinks";
import { SESSION_QUERY_GC_MS, transcriptQueryOptions } from "@/lib/sessionQuery";

type Props = {
  sessionId: string;
  live?: boolean;
  isRunning?: boolean;
  liveEvents?: ChatStreamEvent[];
  /** Optional pre-resolved blocks; when omitted, merges transcript query + liveEvents. */
  blocks?: TranscriptBlock[];
  selectedTool?: TranscriptBlock | null;
  onSelectTool?: (tool: TranscriptBlock | null) => void;
};

export function TracePanel({
  sessionId,
  live,
  isRunning = false,
  liveEvents = [],
  blocks: blocksProp,
  selectedTool,
  onSelectTool,
}: Props) {
  const t = useT();
  const running = Boolean(isRunning);
  const streamLive = Boolean(live);

  const transcript = useQuery({
    ...transcriptQueryOptions(sessionId, running, streamLive, streamLive),
    enabled: Boolean(sessionId) && blocksProp == null,
    placeholderData: (prev) => prev,
    refetchInterval: running && !streamLive ? 5_000 : false,
  });

  const blocks = useMemo(() => {
    if (blocksProp) return blocksProp;
    const snapshot = transcript.data?.transcript.blocks ?? [];
    const snapshotMaxSeq = transcript.data?.transcript.max_seq ?? 0;
    return resolveCanonicalTranscriptBlocks(
      snapshot,
      liveEvents,
      snapshotMaxSeq,
      streamLive,
    );
  }, [
    blocksProp,
    liveEvents,
    streamLive,
    transcript.data?.transcript.blocks,
    transcript.data?.transcript.max_seq,
  ]);

  // Keep /trace warm for debug invalidate; not used as primary timeline.
  useQuery({
    queryKey: ["session-trace-inspector", sessionId],
    queryFn: () => api.sessionTrace(sessionId),
    enabled: Boolean(sessionId),
    staleTime: running ? 3_000 : 15_000,
    gcTime: SESSION_QUERY_GC_MS,
    refetchInterval: running && !live ? 6_000 : false,
    placeholderData: (prev) => prev,
  });

  return (
    <div className="flex flex-col min-h-0 h-full overflow-y-auto">
      <section className="border-b border-outline-variant/60">
        <h3 className="px-3 py-2 text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 flex items-center gap-1.5 bg-surface-container-low/50">
          <Icon name="timeline" size={14} />
          {t("conversations.inspectorTimeline")}
          {blocks.length > 0 && (
            <span className="text-outline normal-case">({blocks.length})</span>
          )}
        </h3>
        {transcript.isPending && blocksProp == null && blocks.length === 0 ? (
          <p className="text-xs text-secondary m-0 px-3 py-2">{t("common.loading")}</p>
        ) : (
          <ExecutionTraceGraph
            blocks={blocks}
            isRunning={running}
            selectedToolId={selectedTool?.id ?? null}
            onSelectTool={onSelectTool}
          />
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
          search={sessionDetailSearch("debug")}
          className="text-xs text-secondary no-underline hover:text-primary inline-flex items-center gap-1"
        >
          <Icon name="timeline" size={14} />
          {t("conversations.openDetail")}
        </Link>
      </div>
    </div>
  );
}
