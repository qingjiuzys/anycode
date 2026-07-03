import { useEffect, useMemo, useState } from "react";
import type { TranscriptBlock } from "@/api/types";
import { Icon } from "@/components/Icon";
import { formatTranscriptBlockTitle } from "@/lib/eventFormat";
import {
  countLogicalToolSteps,
  toolStepFailed,
  toolStepRunning,
  type ToolStep,
} from "@/lib/transcriptGrouping";
import {
  extractToolCommand,
  formatDurationMs,
  formatToolStepLabelParts,
} from "@/lib/toolStepLabel";
import { toolIconName } from "@/lib/toolIcons";
import { useT } from "@/i18n/context";

type Props = {
  steps: ToolStep[];
  processMessageCount?: number;
  processSnippets?: string[];
  isRunning?: boolean;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
};

export function ToolTraceCluster({
  steps,
  processMessageCount = 0,
  processSnippets = [],
  isRunning = false,
  selectedToolId,
  onSelectTool,
}: Props) {
  const t = useT();
  const anyRunning = steps.some(toolStepRunning);
  const anyFailed = steps.some(toolStepFailed);
  const streaming = anyRunning || isRunning;
  const [historyOpen, setHistoryOpen] = useState(false);

  useEffect(() => {
    if (streaming) {
      setHistoryOpen(false);
    }
  }, [streaming]);

  const summary = useMemo(() => {
    const count = countLogicalToolSteps(
      steps.flatMap((step) => [step.call, step.result].filter(Boolean) as TranscriptBlock[]),
    );
    const last = [...steps].reverse().find((step) => step.call || step.result);
    const lastLabel = last
      ? formatToolStepLabelParts(last, (block) => formatTranscriptBlockTitle(block, t)).toolName
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
  }, [steps, t]);

  if (steps.length === 0 && processMessageCount === 0) {
    return null;
  }

  const stripClass = anyFailed
    ? "tool-strip--failed"
    : streaming
      ? "tool-strip--running"
      : "tool-strip--done";

  if (streaming) {
    return (
      <div className={`tool-strip tool-strip-streaming ${stripClass}`}>
        {processMessageCount > 0 && (
          <ThinkingTraceFold
            count={processMessageCount}
            snippets={processSnippets}
            loading
          />
        )}
        {steps.map((step) => (
          <ToolStepLine
            key={step.key}
            step={step}
            selectedToolId={selectedToolId}
            onSelectTool={onSelectTool}
          />
        ))}
      </div>
    );
  }

  return (
    <div className={`tool-strip ${stripClass}`}>
      {processMessageCount > 0 && (
        <ThinkingTraceFold
          count={processMessageCount}
          snippets={processSnippets}
          loading={false}
        />
      )}
      {steps.length > 0 && (
        <>
          <button
            type="button"
            className="tool-strip-summary"
            onClick={() => setHistoryOpen((v) => !v)}
            aria-expanded={historyOpen}
          >
            <Icon name={historyOpen ? "expand_more" : "chevron_right"} size={14} />
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
          </button>
          {historyOpen && (
            <div className="tool-strip-steps">
              {steps.map((step) => (
                <ToolStepLine
                  key={step.key}
                  step={step}
                  selectedToolId={selectedToolId}
                  onSelectTool={onSelectTool}
                  allowExpand
                />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

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
  const [open, setOpen] = useState(loading);

  useEffect(() => {
    if (loading) {
      setOpen(true);
    } else {
      setOpen(false);
    }
  }, [loading]);

  const label = loading
    ? t("conversations.thinkingRunning")
    : count <= 1
      ? t("conversations.thinkingBrief")
      : t("conversations.thinkingDone").replace("{n}", String(count));

  return (
    <div className="tool-strip-step tool-strip-step-thinking">
      <button
        type="button"
        className="tool-strip-step-toggle"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        disabled={loading}
      >
        <Icon name={loading ? "progress_activity" : "psychology"} size={14} />
        <span>{label}</span>
        {!loading && snippets.length > 0 && (
          <Icon name={open ? "expand_more" : "chevron_right"} size={14} />
        )}
      </button>
      {open && !loading && snippets.length > 0 && (
        <div className="tool-strip-step-body">
          <ul className="m-0 pl-4 space-y-1">
            {snippets.map((snippet, i) => (
              <li key={`${i}-${snippet.slice(0, 24)}`} className="text-xs text-secondary">
                {snippet}
              </li>
            ))}
          </ul>
        </div>
      )}
      {open && loading && snippets.length > 0 && (
        <div className="tool-strip-step-body">
          <p className="m-0 text-xs text-secondary">{snippets[snippets.length - 1]}</p>
        </div>
      )}
    </div>
  );
}

function ToolStepLine({
  step,
  selectedToolId,
  onSelectTool,
  allowExpand = true,
}: {
  step: ToolStep;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
  allowExpand?: boolean;
}) {
  const t = useT();
  const primary = step.result ?? step.call;
  const running = toolStepRunning(step);
  const failed = toolStepFailed(step);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    if (running) {
      setExpanded(false);
    }
  }, [running]);

  if (!primary) return null;

  const parts = formatToolStepLabelParts(step, (block) =>
    formatTranscriptBlockTitle(block, t),
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
        />
        <span className="tool-strip-step-name">{parts.toolName}</span>
        {parts.command && (
          <span className="tool-strip-step-command font-code">{parts.command}</span>
        )}
        {parts.duration && <span className="tool-strip-step-duration">{parts.duration}</span>}
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
