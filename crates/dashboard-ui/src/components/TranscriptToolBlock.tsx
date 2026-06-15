import { useEffect, useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import type { TranscriptBlock } from "@/api/types";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/Icon";
import { previewLines } from "@/components/ui/CollapsiblePanel";
import { formatTranscriptBlockTitle } from "@/lib/eventFormat";
import { countLogicalToolSteps, toolStepKey } from "@/lib/transcriptGrouping";
import { useT } from "@/i18n/context";

type Props = {
  tools: TranscriptBlock[];
  processMessageCount?: number;
  compact?: boolean;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock) => void;
};

export function TranscriptToolBlock({
  tools,
  processMessageCount = 0,
  compact = true,
  selectedToolId,
  onSelectTool,
}: Props) {
  const t = useT();
  const failed = tools.some(
    (tool) =>
      tool.block_type === "tool_result" &&
      /failed|error|denied/i.test(`${tool.title} ${tool.body}`),
  );
  const running =
    tools.some((tool) => tool.block_type === "tool_call") &&
    !tools.some((tool) => tool.block_type === "tool_result");

  const summary = useMemo(
    () => buildGroupSummary(tools, processMessageCount, t),
    [tools, processMessageCount, t],
  );
  const [open, setOpen] = useState(false);
  const elapsed = useRunningElapsedSeconds(running ? tools[0]?.at : undefined);

  const stepRows = useMemo(() => summarizeToolSteps(tools), [tools]);

  if (compact) {
    const subtitleParts = [summary.status];
    if (summary.currentTool) {
      subtitleParts.push(summary.currentTool);
    }
    if (running && elapsed != null) {
      subtitleParts.push(`${elapsed}s`);
    }
    if (processMessageCount > 0) {
      subtitleParts.push(
        t("conversations.toolProcessMessages").replace("{n}", String(processMessageCount)),
      );
    }

    return (
      <div
        className={`tool-strip rounded-xl border border-dashed overflow-hidden ${
          failed
            ? "border-error/40 bg-error/5"
            : running
              ? "border-primary/35 bg-primary/5"
              : "border-outline-variant/70 bg-surface-container-low/40"
        }`}
      >
        <button
          type="button"
          className="w-full flex items-center gap-2 px-3 py-2 text-left text-sm bg-transparent border-0 cursor-pointer hover:bg-surface-container/50"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
        >
          <Icon
            name={open ? "expand_less" : "chevron_right"}
            size={16}
            className="text-secondary shrink-0"
          />
          {running && (
            <span
              aria-hidden
              className="inline-block w-1.5 h-1.5 rounded-full bg-primary animate-pulse shrink-0"
            />
          )}
          <span className="font-medium text-on-surface truncate">{summary.title}</span>
          <span className="text-xs text-secondary truncate min-w-0 flex-1">
            {subtitleParts.join(" · ")}
          </span>
          <CopyButton text={summary.combined} label={t("conversations.copyMessage")} />
        </button>
        {open && (
          <ul className="m-0 p-0 list-none border-t border-outline-variant/50">
            {stepRows.slice(0, 5).map((row) => (
              <CompactStepRow
                key={row.id}
                row={row}
                selected={selectedToolId === row.id}
                onSelect={() => {
                  const block = tools.find((tool) => tool.id === row.id);
                  if (block) {
                    onSelectTool?.(block);
                  }
                }}
                t={t}
              />
            ))}
            {stepRows.length > 5 && (
              <li className="px-3 py-1.5 text-[11px] text-secondary">
                {t("conversations.toolStepsMore").replace("{n}", String(stepRows.length - 5))}
              </li>
            )}
          </ul>
        )}
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-outline-variant/60 bg-surface-container-low overflow-hidden">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-outline-variant/50">
        <Icon name="build" size={16} className="text-secondary shrink-0" />
        <span className="font-medium text-sm">{summary.title}</span>
        <span className="text-xs text-secondary truncate flex-1">{summary.status}</span>
        <CopyButton text={summary.combined} label={t("conversations.copyMessage")} />
      </div>
      <ul className="m-0 p-0 list-none">
        {stepRows.map((row) => (
          <CompactStepRow
            key={row.id}
            row={row}
            selected={selectedToolId === row.id}
            onSelect={() => {
              const block = tools.find((tool) => tool.id === row.id);
              if (block) {
                onSelectTool?.(block);
              }
            }}
            t={t}
          />
        ))}
      </ul>
    </div>
  );
}

type StepRow = {
  id: string;
  label: string;
  preview: string;
  failed: boolean;
  isResult: boolean;
};

function CompactStepRow({
  row,
  selected,
  onSelect,
  t,
}: {
  row: StepRow;
  selected: boolean;
  onSelect: () => void;
  t: (key: string) => string;
}) {
  return (
    <li>
      <button
        type="button"
        className={`w-full flex items-center gap-2 px-3 py-1.5 text-left text-xs border-0 cursor-pointer ${
          selected
            ? "bg-primary/10 text-primary"
            : "bg-transparent hover:bg-surface-container/60 text-on-surface"
        }`}
        onClick={onSelect}
      >
        <Icon
          name={row.isResult ? (row.failed ? "error" : "check_circle") : "edit"}
          size={14}
          className={row.failed ? "text-error shrink-0" : "text-secondary shrink-0"}
        />
        <span className="font-medium truncate">{row.label}</span>
        {row.preview && (
          <span className="text-secondary truncate min-w-0 flex-1 font-code">{row.preview}</span>
        )}
        <span className="text-[10px] text-secondary shrink-0">{t("conversations.viewDetail")}</span>
      </button>
    </li>
  );
}

function summarizeToolSteps(tools: TranscriptBlock[]): StepRow[] {
  const byKey = new Map<string, { call?: TranscriptBlock; result?: TranscriptBlock }>();
  const order: string[] = [];

  for (const tool of tools) {
    const key = toolStepKey(tool) ?? tool.id;
    if (!byKey.has(key)) {
      order.push(key);
      byKey.set(key, {});
    }
    const slot = byKey.get(key)!;
    if (tool.block_type === "tool_result") {
      slot.result = tool;
    } else {
      slot.call = tool;
    }
  }

  return order.map((key) => {
    const slot = byKey.get(key)!;
    const primary = slot.result ?? slot.call!;
    const failed = /failed|error|denied/i.test(`${primary.title} ${primary.body}`);
    return {
      id: primary.id,
      label: primary.title,
      preview: previewLines(primary.body, 1, 96),
      failed,
      isResult: primary.block_type === "tool_result",
    };
  });
}

function useRunningElapsedSeconds(startedAt?: string): number | null {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!startedAt) return;
    const id = setInterval(() => setNow(Date.now()), 1_000);
    return () => clearInterval(id);
  }, [startedAt]);
  if (!startedAt) return null;
  const start = new Date(startedAt).getTime();
  if (Number.isNaN(start)) return null;
  return Math.max(0, Math.floor((now - start) / 1000));
}

function buildGroupSummary(
  tools: TranscriptBlock[],
  processMessageCount: number,
  t: (key: string) => string,
): {
  title: string;
  status: string;
  currentTool: string;
  combined: string;
  eventId?: string;
} {
  const calls = tools.filter((x) => x.block_type === "tool_call").length;
  const results = tools.filter((x) => x.block_type === "tool_result").length;
  const failed = tools.some(
    (x) =>
      x.block_type === "tool_result" && /failed|error|denied/i.test(`${x.title} ${x.body}`),
  );
  const done = results >= calls && calls > 0;
  const stepCount = countLogicalToolSteps(tools);
  const title = t("conversations.toolStripTitle")
    .replace("{tools}", String(stepCount))
    .replace("{process}", String(processMessageCount));
  const status = failed
    ? t("conversations.toolGroupFailed")
    : done
      ? t("conversations.toolGroupDone")
      : t("conversations.toolGroupRunning");
  const runningCall = [...tools].reverse().find((x) => x.block_type === "tool_call");
  const currentTool = runningCall?.title ?? tools[tools.length - 1]?.title ?? "";
  const combined = tools
    .map((tool) => `${tool.title}\n${tool.body}`.trim())
    .filter(Boolean)
    .join("\n\n");
  return {
    title,
    status,
    currentTool,
    combined,
    eventId: tools.find((x) => x.event_id)?.event_id ?? undefined,
  };
}

export function ToolDetailPanel({ tool }: { tool: TranscriptBlock | null }) {
  const t = useT();
  if (!tool) {
    return (
      <p className="text-xs text-secondary m-0 px-3 py-2">
        {t("conversations.inspectorSelectTool")}
      </p>
    );
  }

  const label = formatTranscriptBlockTitle(tool, t);
  const meta = tool.meta ?? {};
  const command =
    typeof meta.command === "string"
      ? meta.command
      : previewLines(tool.body, 1, 200);

  return (
    <div className="px-3 py-2 space-y-2">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="m-0 text-sm font-medium truncate">{label}</p>
          <p className="m-0 text-[11px] text-secondary">
            {typeof meta.phase === "string" ? meta.phase : tool.block_type}
            {typeof meta.duration_ms === "string" ? ` · ${meta.duration_ms}ms` : ""}
          </p>
        </div>
        {tool.event_id && (
          <Link
            to="/events/$eventId"
            params={{ eventId: tool.event_id }}
            className="dw-btn-ghost text-[10px] py-0.5 no-underline shrink-0"
          >
            <Icon name="link" size={12} />
          </Link>
        )}
      </div>
      {command && (
        <div>
          <p className="m-0 text-[10px] uppercase tracking-wide text-secondary mb-1">
            {t("conversations.inspectorInput")}
          </p>
          <pre className="m-0 text-xs font-code whitespace-pre-wrap break-words bg-surface-container-low rounded-lg p-2 border border-outline-variant/50 max-h-40 overflow-y-auto">
            {command}
          </pre>
        </div>
      )}
      {tool.body && (
        <div>
          <p className="m-0 text-[10px] uppercase tracking-wide text-secondary mb-1">
            {t("conversations.inspectorOutput")}
          </p>
          <pre className="m-0 text-xs font-code whitespace-pre-wrap break-words bg-surface-container-low rounded-lg p-2 border border-outline-variant/50 max-h-64 overflow-y-auto">
            {tool.body}
          </pre>
        </div>
      )}
      {typeof meta.error === "string" && meta.error !== "<none>" && (
        <p className="m-0 text-xs text-error">{meta.error}</p>
      )}
    </div>
  );
}
