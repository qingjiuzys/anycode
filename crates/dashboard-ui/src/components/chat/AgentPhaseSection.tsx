import { memo, useState, type ReactNode } from "react";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { TranscriptMarkdown } from "@/components/TranscriptMarkdown";
import { ToolTraceCluster } from "@/components/chat/ToolTraceCluster";
import { sanitizeAssistantDisplay } from "@/lib/assistantText";
import {
  phaseTitleKey,
  progressDiscovery,
  progressNext,
  progressPhase,
  progressSummary,
  progressWorkStage,
  workStageLabelKey,
  type AgentPhaseKind,
} from "@/lib/progressMeta";
import type { AgentPhaseSegment } from "@/lib/phaseGrouping";
import { useLocale, useT } from "@/i18n/context";

type Props = {
  segment: AgentPhaseSegment;
  defaultExpanded?: boolean;
  isRunning?: boolean;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
  suppressActivityLine?: boolean;
  toolDefaultCollapsed?: boolean;
  renderDeliver?: (block: TranscriptBlock) => ReactNode;
};

export const AgentPhaseSection = memo(function AgentPhaseSection({
  segment,
  defaultExpanded = true,
  isRunning = false,
  selectedToolId,
  onSelectTool,
  suppressActivityLine = false,
  toolDefaultCollapsed = true,
  renderDeliver,
}: Props) {
  const t = useT();
  const locale = useLocale();
  const [collapsed, setCollapsed] = useState(!defaultExpanded);

  if (segment.deliverBlock && renderDeliver) {
    return (
      <section className="agent-phase-section agent-phase-section--deliver" data-phase="deliver">
        <div className="agent-phase-section__label">{t("conversations.progressPhaseDeliver")}</div>
        {renderDeliver(segment.deliverBlock)}
      </section>
    );
  }

  const block = segment.progressBlock;
  if (!block) {
    if (segment.toolCluster) {
      return (
        <ToolTraceCluster
          steps={segment.toolCluster.steps}
          processMessageCount={segment.toolCluster.processMessageCount}
          processSnippets={segment.toolCluster.processSnippets}
          isRunning={isRunning}
          selectedToolId={selectedToolId}
          onSelectTool={onSelectTool}
          suppressActivityLine={suppressActivityLine}
          defaultCollapsed={toolDefaultCollapsed}
        />
      );
    }
    return null;
  }

  const phase: AgentPhaseKind = progressPhase(block);
  const live = Boolean(block.meta?.live);
  const summary = progressSummary(block, sanitizeAssistantDisplay(block.body, locale));
  const next = progressNext(block);
  const discovery = progressDiscovery(block);
  const workStage = progressWorkStage(block);
  const workStageKey = workStage ? workStageLabelKey(workStage) : null;

  if (!summary && !next && !discovery) {
    return null;
  }

  const expanded = live || !collapsed;

  return (
    <section
      className={`agent-phase-section agent-progress-card ${live ? "agent-progress-card--live" : ""}`}
      data-phase={phase}
      data-testid="agent-phase-section"
    >
      <button
        type="button"
        className="agent-progress-card__heading agent-phase-section__toggle"
        onClick={() => {
          if (!live) setCollapsed((v) => !v);
        }}
        aria-expanded={expanded}
        aria-label={expanded ? t("common.collapse") : t("common.expand")}
        disabled={live}
      >
        {!live && (
          <Icon
            name={expanded ? "expand_more" : "chevron_right"}
            size={18}
            className="transcript-expand-icon shrink-0"
          />
        )}
        <Icon name={live ? "progress_activity" : "route"} size={14} className={live ? "animate-spin" : ""} />
        <span>{t(phaseTitleKey(phase))}</span>
        {workStageKey && (
          <span className="agent-phase-section__work-stage">{t(workStageKey)}</span>
        )}
        {live && <span className="agent-progress-card__live-dot" aria-hidden />}
      </button>

      {(expanded || live) && (
        <div className="agent-progress-card__body">
          {summary && <TranscriptMarkdown text={summary} live={live} />}
          {discovery && (
            <p className="agent-phase-section__discovery m-0 mt-1.5 text-sm">
              <span className="font-medium">{t("conversations.progressDiscoveryPrefix")}</span>
              {discovery}
            </p>
          )}
          {next && (
            <p className="agent-phase-section__next m-0 mt-1.5 text-sm text-secondary">
              <span className="font-medium text-on-surface">{t("conversations.progressNextPrefix")}</span>
              {next}
            </p>
          )}
        </div>
      )}

      {!expanded && !live && summary && (
        <p className="agent-phase-section__preview m-0 mt-1 text-xs text-secondary line-clamp-2">
          {summary}
        </p>
      )}

      {segment.toolCluster && (
        <div className="agent-phase-section__evidence mt-2">
          <ToolTraceCluster
            steps={segment.toolCluster.steps}
            processMessageCount={segment.toolCluster.processMessageCount}
            processSnippets={segment.toolCluster.processSnippets}
            isRunning={isRunning}
            selectedToolId={selectedToolId}
            onSelectTool={onSelectTool}
            suppressActivityLine={suppressActivityLine}
            defaultCollapsed={toolDefaultCollapsed}
          />
        </div>
      )}
    </section>
  );
});

export function ExecutionRecordFold({
  count,
  children,
}: {
  count: number;
  children: ReactNode;
}) {
  const t = useT();
  const [open, setOpen] = useState(false);
  if (count <= 0) return null;

  return (
    <div className="execution-record-fold" data-testid="execution-record-fold">
      <button
        type="button"
        className="execution-record-fold__toggle"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
      >
        <Icon name={open ? "expand_more" : "chevron_right"} size={14} />
        <span>{t("conversations.executionRecordFold").replace("{n}", String(count))}</span>
      </button>
      {open && <div className="execution-record-fold__body">{children}</div>}
    </div>
  );
}
