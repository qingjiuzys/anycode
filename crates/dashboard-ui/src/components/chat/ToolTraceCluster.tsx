import { memo, useEffect, useMemo, useState } from "react";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { AgentActivityLine } from "@/components/chat/AgentActivityLine";
import { formatTranscriptBlockTitle } from "@/lib/eventFormat";
import { truncateThinkingPreview } from "@/lib/agentActivitySummary";
import {
  countLogicalToolSteps,
  toolStepFailed,
  toolStepRunning,
  type ToolStep,
} from "@/lib/transcriptGrouping";
import { formatToolFailureRecovery } from "@/lib/agentActivitySummary";
import {
  toolTraceShowThinkingHeader,
  toolTraceStreaming,
} from "@/lib/toolTraceState";
import {
  extractToolCommand,
  formatDurationMs,
  formatToolStepLabelParts,
} from "@/lib/toolStepLabel";
import { toolIconName } from "@/lib/toolIcons";
import { useT } from "@/i18n/context";
import { useSmoothText } from "@/hooks/useSmoothText";
import { formatDeliveryPreflight } from "@/lib/progressMeta";

type Props = {
  steps: ToolStep[];
  processMessageCount?: number;
  processSnippets?: string[];
  isRunning?: boolean;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
  suppressActivityLine?: boolean;
  defaultCollapsed?: boolean;
  /** Accordion: force open/closed from parent (overrides local history toggle when set). */
  forceExpanded?: boolean;
  variant?: "nested" | "flat";
};

export const ToolTraceCluster = memo(function ToolTraceCluster({
  steps,
  processMessageCount = 0,
  processSnippets = [],
  isRunning = false,
  selectedToolId,
  onSelectTool,
  suppressActivityLine = false,
  defaultCollapsed: _defaultCollapsed = true,
  forceExpanded,
  variant = "nested",
}: Props) {
  const t = useT();
  const anyFailed = steps.some(toolStepFailed);
  const streaming = toolTraceStreaming(steps, processMessageCount, isRunning);

  const summary = useMemo(() => {
    const count = countLogicalToolSteps(
      steps.flatMap((step) => [step.call, step.result].filter(Boolean) as TranscriptBlock[]),
    );
    const last = [...steps].reverse().find((step) => step.call || step.result);
    const lastLabel = last
      ? formatToolStepLabelParts(
          last,
          (block) => formatTranscriptBlockTitle(block, t),
          { segmentActive: isRunning },
        ).toolName
      : "";
    const totalMs = steps.reduce((acc, step) => {
      const meta = (step.result?.meta ?? step.call?.meta ?? {}) as Record<string, unknown>;
      const raw = meta.duration_ms ?? meta.elapsed_ms;
      const ms =
        typeof raw === "string"
          ? Number.parseInt(raw, 10)
          : typeof raw === "number"
            ? raw
            : 0;
      return acc + (Number.isNaN(ms) ? 0 : ms);
    }, 0);
    return { count, lastLabel, totalDuration: formatDurationMs({ duration_ms: totalMs }) };
  }, [steps, t, isRunning]);

  if (steps.length === 0 && processMessageCount === 0) {
    return null;
  }

  // Settled cluster carrying only folded empty notices (no steps, no thinking
  // snippets) has nothing to render — avoid an empty tool-strip pill.
  if (steps.length === 0 && processSnippets.length === 0 && !streaming) {
    return null;
  }

  const stripClass = anyFailed
    ? "tool-strip--failed"
    : streaming
      ? "tool-strip--running"
      : "tool-strip--done";

  const failedStep = steps.find(toolStepFailed);
  const recoveryLine = failedStep ? formatToolFailureRecovery(failedStep, t) : null;
  // Codex/Cursor: only the live/streaming cluster shows step rows.
  // Settled success clusters stay a single non-interactive summary line.
  const showStepDetails = streaming && forceExpanded !== false;

  if (variant === "flat") {
    if (steps.length === 0) return null;
    return (
      <div className="tool-strip-flat" data-testid="tool-strip-flat">
        {steps.map((step) => (
          <ToolStepLine
            key={step.key}
            step={step}
            selectedToolId={selectedToolId}
            onSelectTool={onSelectTool}
            allowExpand={false}
            segmentActive={isRunning}
          />
        ))}
        {recoveryLine && (
          <p className="tool-strip-recovery m-0 px-1 py-2 text-xs text-warn" data-testid="tool-failure-recovery">
            {recoveryLine}
          </p>
        )}
      </div>
    );
  }

  if (streaming) {
    const showThinkingHeader = toolTraceShowThinkingHeader(
      steps,
      processMessageCount,
      isRunning,
    );
    return (
      <div className="flex flex-col gap-1.5 w-full max-w-[min(100%,42rem)]">
        {!suppressActivityLine && steps.length > 0 && (
          <AgentActivityLine steps={steps} suppressDuration />
        )}
        <div className={`tool-strip tool-strip-streaming ${stripClass}`}>
          {steps.length > 0 && (
            <div className="tool-strip-summary" role="status">
              <Icon name="progress_activity" size={14} className="text-primary animate-spin" />
              <span>
                {summary.lastLabel
                  ? t("conversations.toolTraceRunningTool").replace(
                      "{tool}",
                      summary.lastLabel,
                    )
                  : t("conversations.agentWorking")}
              </span>
            </div>
          )}
          {showThinkingHeader && (
            <ThinkingTraceFold
              count={processMessageCount}
              snippets={processSnippets}
              loading
            />
          )}
          {showStepDetails &&
            steps.map((step) => (
              <ToolStepLine
                key={step.key}
                step={step}
                selectedToolId={selectedToolId}
                onSelectTool={onSelectTool}
                allowExpand={false}
                segmentActive={isRunning}
              />
            ))}
          {recoveryLine && (
            <p className="tool-strip-recovery m-0 px-3 py-2 text-xs text-warn" data-testid="tool-failure-recovery">
              {recoveryLine}
            </p>
          )}
        </div>
      </div>
    );
  }

  const showThinkingFold =
    processSnippets.length > 0 || processMessageCount > 0;

  return (
    <div className="flex flex-col gap-1.5 w-full max-w-[min(100%,42rem)]">
      {!suppressActivityLine && steps.length > 0 && (
        <AgentActivityLine steps={steps} suppressDuration />
      )}
      <div className={`tool-strip ${stripClass}`}>
        {showThinkingFold && (
          <ThinkingTraceFold
            count={processMessageCount}
            snippets={processSnippets}
            loading={false}
          />
        )}
        {steps.length > 0 && (
          <>
            <div className="tool-strip-summary" role="status" data-testid="tool-strip-summary-done">
              {anyFailed ? (
                <span>{t("conversations.toolTraceFailed").replace("{n}", String(summary.count))}</span>
              ) : (
                <span>{t("conversations.toolTraceDone").replace("{n}", String(summary.count))}</span>
              )}
              {summary.lastLabel && (
                <span className="tool-strip-summary-meta">{summary.lastLabel}</span>
              )}
              {summary.totalDuration && (
                <span className="tool-strip-summary-meta">{summary.totalDuration}</span>
              )}
            </div>
            {recoveryLine && (
              <p className="tool-strip-recovery m-0 px-3 py-2 text-xs text-warn" data-testid="tool-failure-recovery">
                {recoveryLine}
              </p>
            )}
          </>
        )}
      </div>
    </div>
  );
}, (prev, next) =>
  prev.steps === next.steps &&
  prev.processMessageCount === next.processMessageCount &&
  prev.processSnippets === next.processSnippets &&
  prev.isRunning === next.isRunning &&
  prev.selectedToolId === next.selectedToolId &&
  prev.onSelectTool === next.onSelectTool &&
  prev.suppressActivityLine === next.suppressActivityLine &&
  prev.defaultCollapsed === next.defaultCollapsed &&
  prev.forceExpanded === next.forceExpanded &&
  prev.variant === next.variant);

function ThinkingTraceFold({
  count,
  snippets,
  loading,
}: {
  count: number;
  snippets: string[];
  loading: boolean;
}) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const latestSnippet = snippets[snippets.length - 1] ?? "";
  const { text: smoothedSnippet } = useSmoothText(
    `thinking:${count}:${snippets.length}`,
    latestSnippet,
    loading && latestSnippet.length > 0,
  );
  const previewText = truncateThinkingPreview(
    loading ? smoothedSnippet : latestSnippet,
  );

  useEffect(() => {
    if (loading) {
      setOpen(false);
    }
  }, [loading]);

  if (!loading && snippets.length === 0) {
    return null;
  }

  const label = loading
    ? t("conversations.thinkingRunning")
    : count <= 1
      ? t("conversations.thinkingBrief")
      : t("conversations.thinkingDone").replace("{n}", String(count));

  const showPreview = !open && previewText.length > 0;
  const showBulletList = open && !loading && snippets.length > 0;

  return (
    <div className="tool-strip-step tool-strip-step-thinking">
      <button
        type="button"
        className="tool-strip-step-toggle"
        onClick={() => {
          if (!loading && snippets.length > 0) {
            setOpen((v) => !v);
          }
        }}
        aria-expanded={open || showPreview}
        disabled={loading || snippets.length === 0}
      >
        <Icon name={loading ? "progress_activity" : "psychology"} size={14} />
        <span>{label}</span>
        {!loading && snippets.length > 0 && (
          <>
            <span className="tool-strip-step-thinking-expand">
              {open ? t("conversations.thinkingCollapse") : t("conversations.thinkingExpand")}
            </span>
            <Icon name={open ? "expand_more" : "chevron_right"} size={14} />
          </>
        )}
      </button>
      {showPreview && (
        <p className="tool-strip-step-thinking-preview">{previewText}</p>
      )}
      {showBulletList && (
        <div className="tool-strip-step-body">
          <ul className="m-0 pl-4 space-y-1">
            {snippets.map((snippet, i) => (
              <li key={`${i}-${snippet.slice(0, 24)}`} className="text-xs text-secondary">
                {formatDeliveryPreflight(snippet) ?? snippet}
              </li>
            ))}
          </ul>
        </div>
      )}
      {loading && snippets.length > 0 && (
        <p className="tool-strip-step-thinking-preview">{smoothedSnippet}</p>
      )}
    </div>
  );
}

function ToolStepLine({
  step,
  selectedToolId,
  onSelectTool,
  allowExpand = true,
  segmentActive = true,
}: {
  step: ToolStep;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
  allowExpand?: boolean;
  /** When false, unpaired call-without-result must not spin. */
  segmentActive?: boolean;
}) {
  const t = useT();
  const primary = step.result ?? step.call;
  const unpaired = toolStepRunning(step);
  const running = unpaired && segmentActive;
  const abandoned = unpaired && !segmentActive;
  const failed = toolStepFailed(step) || abandoned;
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    if (running) {
      setExpanded(false);
    }
  }, [running]);

  if (!primary) return null;

  const parts = formatToolStepLabelParts(
    step,
    (block) => formatTranscriptBlockTitle(block, t),
    { segmentActive },
  );
  const toolName =
    (primary.meta?.name as string | undefined) ??
    primary.title.replace(/\s+(started|finished|failed)$/i, "").trim();
  const callBody = extractToolCommand(step.call);
  const resultBody = primary.body?.trim() ?? "";
  const expandBody = [callBody, resultBody].filter(Boolean).join("\n\n");
  const hasBody = expandBody.length > 0;
  const canExpand = allowExpand && !running && hasBody;
  const showBody = canExpand && expanded;

  return (
    <div
      className={`tool-strip-step ${
        running
          ? "tool-strip-step-active"
          : failed
            ? "tool-strip-step-failed"
            : "tool-strip-step-done"
      }`}
    >
      <button
        type="button"
        className="tool-strip-step-toggle"
        onClick={() => {
          if (canExpand) {
            setExpanded((v) => !v);
          }
          onSelectTool?.(primary);
        }}
        aria-expanded={showBody}
        data-selected={selectedToolId === primary.id ? "true" : undefined}
        disabled={!canExpand && !onSelectTool}
      >
        <Icon
          name={
            failed
              ? "error"
              : running
                ? "progress_activity"
                : toolIconName(toolName)
          }
          size={14}
          className={running ? "animate-spin text-primary" : undefined}
        />
        <span className="tool-strip-step-name">{parts.toolName}</span>
        {parts.command && (
          <span className="tool-strip-step-command font-code">{parts.command}</span>
        )}
        {parts.duration && <span className="tool-strip-step-duration">{parts.duration}</span>}
        {abandoned && (
          <span className="tool-strip-step-duration">
            {t("conversations.toolTraceAbandoned")}
          </span>
        )}
        {!running && (
          <span className="tool-strip-step-status" aria-hidden>
            {failed ? "✗" : "✓"}
          </span>
        )}
        {canExpand && <Icon name={expanded ? "expand_more" : "chevron_right"} size={14} />}
      </button>
      {showBody && <pre className="tool-strip-step-body">{expandBody}</pre>}
    </div>
  );
}
