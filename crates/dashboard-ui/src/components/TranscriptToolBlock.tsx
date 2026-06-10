import { useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import type { TranscriptBlock } from "@/api/types";
import { CollapsiblePanel, contentStats, previewLines } from "@/components/ui/CollapsiblePanel";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/Icon";
import { formatTranscriptBlockTitle } from "@/lib/eventFormat";
import { useT } from "@/i18n/context";

type Props = {
  tools: TranscriptBlock[];
  defaultCollapsed?: boolean;
};

export function TranscriptToolBlock({ tools, defaultCollapsed }: Props) {
  const t = useT();
  const failed = tools.some(
    (tool) =>
      tool.block_type === "tool_result" &&
      /failed|error|denied/i.test(`${tool.title} ${tool.body}`),
  );
  const running =
    tools.some((tool) => tool.block_type === "tool_call") &&
    !tools.some((tool) => tool.block_type === "tool_result");

  const summary = useMemo(() => buildGroupSummary(tools, t), [tools, t]);
  const initialOpen = defaultCollapsed === false && !running;

  return (
    <CollapsiblePanel
      title={summary.title}
      subtitle={summary.subtitle}
      defaultOpen={initialOpen}
      tone={failed ? "error" : running ? "running" : "muted"}
      icon="build"
      headerActions={
        <>
          <CopyButton text={summary.combined} label={t("conversations.copyMessage")} />
          {summary.eventId && (
            <Link
              to="/events/$eventId"
              params={{ eventId: summary.eventId }}
              className="dw-btn-ghost text-[10px] py-0.5 no-underline"
            >
              <Icon name="link" size={12} />
            </Link>
          )}
        </>
      }
    >
      <ul className="m-0 p-0 list-none flex flex-col gap-1">
        {tools.map((tool) => (
          <ToolStepRow key={tool.id} tool={tool} />
        ))}
      </ul>
    </CollapsiblePanel>
  );
}

function ToolStepRow({ tool }: { tool: TranscriptBlock }) {
  const t = useT();
  const failed = /failed|error|denied/i.test(`${tool.title} ${tool.body}`);
  const isResult = tool.block_type === "tool_result";
  const { lines, chars } = contentStats(tool.body);
  const collapsible = tool.collapsible !== false && (lines > 4 || chars > 200);
  const defaultOpen =
    tool.default_collapsed === false ||
    (!isResult && tool.default_collapsed !== true && lines <= 3);
  const [open, setOpen] = useState(defaultOpen);

  const label = formatTranscriptBlockTitle(tool, t);
  const preview = previewLines(tool.body, 1, 120);

  return (
    <li className="rounded-lg border border-outline-variant/50 bg-surface-container-low overflow-hidden">
      <button
        type="button"
        className="w-full flex items-center gap-2 px-2.5 py-2 text-left text-sm bg-transparent border-0 cursor-pointer hover:bg-surface-container/60"
        onClick={() => (collapsible ? setOpen((v) => !v) : undefined)}
        aria-expanded={collapsible ? open : undefined}
        disabled={!collapsible}
      >
        <Icon
          name={isResult ? (failed ? "error" : "check_circle") : "edit"}
          size={16}
          className={failed ? "text-error shrink-0" : "text-secondary shrink-0"}
        />
        <span className="flex-1 min-w-0 font-medium truncate">{label}</span>
        {chars > 0 && (
          <span className="text-[10px] font-code text-secondary shrink-0 tabular-nums">
            {lines > 0 ? `${lines}L` : ""}
            {lines > 0 && chars > 0 ? " · " : ""}
            {chars > 999 ? `${Math.round(chars / 1000)}k` : chars}
          </span>
        )}
        {collapsible && (
          <Icon name={open ? "expand_less" : "expand_more"} size={16} className="text-secondary shrink-0" />
        )}
      </button>
      {(!collapsible || open) && tool.body && (
        <pre className="m-0 px-2.5 pb-2.5 pt-0 text-xs font-code whitespace-pre-wrap break-words text-on-surface border-t border-outline-variant/40">
          {tool.body}
        </pre>
      )}
      {collapsible && !open && preview && (
        <p className="m-0 px-2.5 pb-2 text-xs text-secondary truncate">{preview}</p>
      )}
    </li>
  );
}

function buildGroupSummary(
  tools: TranscriptBlock[],
  t: (key: string) => string,
): { title: string; subtitle: string; combined: string; eventId?: string } {
  const calls = tools.filter((x) => x.block_type === "tool_call").length;
  const results = tools.filter((x) => x.block_type === "tool_result").length;
  const failed = tools.some(
    (x) =>
      x.block_type === "tool_result" && /failed|error|denied/i.test(`${x.title} ${x.body}`),
  );
  const done = results >= calls && calls > 0;
  const title = t("conversations.toolGroupTitle").replace("{n}", String(tools.length));
  const status = failed
    ? t("conversations.toolGroupFailed")
    : done
      ? t("conversations.toolGroupDone")
      : t("conversations.toolGroupRunning");
  const first = tools.find((x) => x.block_type === "tool_call")?.title ?? tools[0]?.title ?? "";
  const subtitle = first ? `${status} · ${first}` : status;
  const combined = tools
    .map((tool) => `${tool.title}\n${tool.body}`.trim())
    .filter(Boolean)
    .join("\n\n");
  return {
    title,
    subtitle,
    combined,
    eventId: tools.find((x) => x.event_id)?.event_id ?? undefined,
  };
}
