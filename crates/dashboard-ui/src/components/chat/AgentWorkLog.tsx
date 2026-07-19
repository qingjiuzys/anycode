import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { ToolTraceCluster } from "@/components/chat/ToolTraceCluster";
import { sanitizeAssistantDisplay } from "@/lib/assistantText";
import { countLogicalToolSteps, toolStepRunning } from "@/lib/transcriptGrouping";
import { progressDiscovery, progressNext, progressSummary } from "@/lib/progressMeta";
import type { AgentTurnWorkBundle } from "@/lib/workLogGrouping";
import { useLocale, useT } from "@/i18n/context";

type Props = {
  work: AgentTurnWorkBundle;
  isRunning?: boolean;
  isLast?: boolean;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
};

export const AgentWorkLog = memo(function AgentWorkLog({
  work,
  isRunning = false,
  isLast = false,
  selectedToolId,
  onSelectTool,
}: Props) {
  const t = useT();
  const locale = useLocale();
  const live = isRunning && isLast;
  const [expanded, setExpanded] = useState(live);
  const streamRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setExpanded(live);
  }, [live]);

  useEffect(() => {
    if (!live || !streamRef.current) return;
    streamRef.current.scrollTop = streamRef.current.scrollHeight;
  }, [live, work.progressLines.length, work.toolSteps.length]);

  const toolCount = useMemo(
    () =>
      countLogicalToolSteps(
        work.toolSteps.flatMap((step) =>
          [step.call, step.result].filter(Boolean) as TranscriptBlock[],
        ),
      ),
    [work.toolSteps],
  );

  const latestSummary = useMemo(() => {
    for (let i = work.progressLines.length - 1; i >= 0; i -= 1) {
      const block = work.progressLines[i]!;
      const text = progressSummary(block, sanitizeAssistantDisplay(block.body, locale));
      if (text) return text;
    }
    for (let i = work.discoveries.length - 1; i >= 0; i -= 1) {
      const block = work.discoveries[i]!;
      const text = progressSummary(block, sanitizeAssistantDisplay(block.body, locale));
      if (text) return text;
    }
    return work.processSnippets[work.processSnippets.length - 1]?.trim() ?? null;
  }, [work.discoveries, work.processSnippets, work.progressLines, locale]);

  const runningTool = work.toolSteps.find(toolStepRunning);
  const fallbackToolLabel =
    runningTool?.call?.title?.replace(/\s+(started|finished|failed)$/i, "").trim() ||
    work.toolSteps
      .map((step) => step.call?.title?.replace(/\s+(started|finished|failed)$/i, "").trim())
      .find((name) => Boolean(name));
  const hasContent =
    work.progressLines.length > 0 ||
    work.discoveries.length > 0 ||
    work.toolSteps.length > 0 ||
    work.processSnippets.length > 0;

  if (!hasContent) {
    return null;
  }

  const summaryLine =
    latestSummary ??
    (live && fallbackToolLabel
      ? t("conversations.progressRunningTool").replace("{tool}", fallbackToolLabel)
      : null) ??
    (toolCount > 0
      ? t("conversations.toolTraceDone").replace("{n}", String(toolCount))
      : t("conversations.agentWorking"));

  if (!live && !expanded) {
    return (
      <section className="agent-work-log" data-testid="agent-work-log">
        <button
          type="button"
          className="agent-work-summary"
          onClick={() => setExpanded(true)}
          aria-expanded={false}
        >
          <Icon name="chevron_right" size={14} />
          <span className="agent-work-summary__text">{summaryLine}</span>
          {toolCount > 0 && (
            <span className="agent-work-summary__meta">
              {t("conversations.toolTraceDone").replace("{n}", String(toolCount))}
            </span>
          )}
        </button>
      </section>
    );
  }

  return (
    <section
      className={`agent-work-log ${live ? "agent-work-log--live" : ""}`}
      data-testid="agent-work-log"
    >
      {!live && (
        <button
          type="button"
          className="agent-work-summary agent-work-summary--open"
          onClick={() => setExpanded(false)}
          aria-expanded
        >
          <Icon name="expand_more" size={14} />
          <span className="agent-work-summary__text">{summaryLine}</span>
          {toolCount > 0 && (
            <span className="agent-work-summary__meta">
              {t("conversations.toolTraceDone").replace("{n}", String(toolCount))}
            </span>
          )}
        </button>
      )}

      <div
        ref={streamRef}
        className={`agent-work-stream ${live ? "agent-work-stream--live" : ""}`}
      >
        {live && work.progressLines.length === 0 && work.discoveries.length === 0 && (
          <div className="agent-work-line agent-work-line--live">
            <p className="m-0 text-sm">{summaryLine}</p>
          </div>
        )}
        {work.progressLines.map((block) => (
          <WorkLine key={block.id} block={block} locale={locale} live={live} />
        ))}
        {work.discoveries.map((block) => (
          <WorkLine key={block.id} block={block} locale={locale} live={false} discovery />
        ))}
        {live && runningTool && latestSummary && (
          <p className="agent-work-stream__running m-0 text-sm text-primary">
            <Icon name="progress_activity" size={14} className="inline animate-spin mr-1" />
            {runningTool.call?.title?.replace(/\s+started$/i, "") ??
              t("conversations.agentWorking")}
          </p>
        )}
      </div>

      {work.toolSteps.length > 0 && (
        <div className="agent-work-tools">
          <ToolTraceCluster
            variant="flat"
            steps={work.toolSteps}
            processMessageCount={0}
            processSnippets={[]}
            isRunning={live && Boolean(runningTool)}
            selectedToolId={selectedToolId}
            onSelectTool={onSelectTool}
            suppressActivityLine
            defaultCollapsed={false}
          />
        </div>
      )}
    </section>
  );
});

function WorkLine({
  block,
  locale,
  live,
  discovery = false,
}: {
  block: TranscriptBlock;
  locale: ReturnType<typeof useLocale>;
  live: boolean;
  discovery?: boolean;
}) {
  const t = useT();
  const summary = progressSummary(block, sanitizeAssistantDisplay(block.body, locale));
  const next = progressNext(block);
  const finding = progressDiscovery(block);
  if (!summary && !next && !finding) return null;

  return (
    <div className={`agent-work-line ${live ? "agent-work-line--live" : ""}`}>
      {summary && <p className="m-0 text-sm">{summary}</p>}
      {finding && (
        <p className="m-0 mt-1 text-xs text-secondary">
          <span className="font-medium">{t("conversations.progressDiscoveryPrefix")}</span>
          {finding}
        </p>
      )}
      {next && !discovery && (
        <p className="m-0 mt-1 text-xs text-secondary">
          <span className="font-medium">{t("conversations.progressNextPrefix")}</span>
          {next}
        </p>
      )}
    </div>
  );
}
