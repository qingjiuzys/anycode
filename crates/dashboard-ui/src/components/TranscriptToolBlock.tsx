import { useState } from "react";
import { Link } from "@tanstack/react-router";
import type { TranscriptBlock } from "@/api/types";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  tools: TranscriptBlock[];
  defaultCollapsed?: boolean;
};

export function TranscriptToolBlock({ tools, defaultCollapsed }: Props) {
  const t = useT();
  const collapsible = tools.some((tool) => tool.collapsible) || tools.length > 1;
  const initialCollapsed =
    defaultCollapsed !== undefined
      ? defaultCollapsed
      : tools.some((tool) => tool.default_collapsed) || tools.length > 2;
  const [open, setOpen] = useState(!initialCollapsed);

  const title =
    tools.find((b) => b.block_type === "tool_call")?.title ||
    tools[0]?.title ||
    t("conversations.toolActivity");

  const failed = tools.some(
    (tool) =>
      tool.block_type === "tool_result" &&
      /failed|error|denied/i.test(`${tool.title} ${tool.body}`),
  );
  const running =
    tools.some((tool) => tool.block_type === "tool_call") &&
    !tools.some((tool) => tool.block_type === "tool_result");

  const combinedBody = tools
    .map((tool) => {
      const head = tool.title ? `# ${tool.title}\n` : "";
      return `${head}${tool.body}`.trim();
    })
    .filter(Boolean)
    .join("\n\n");

  const eventId = tools.find((tool) => tool.event_id)?.event_id;

  return (
    <div
      className={`rounded-2xl rounded-bl-md border overflow-hidden w-full ${
        failed
          ? "border-error/30 bg-error-container/15"
          : running
            ? "border-primary/25 bg-primary-container/15"
            : "border-outline-variant bg-surface-container-low"
      }`}
    >
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-outline-variant/60">
        {running && (
          <span className="inline-block w-2 h-2 rounded-full bg-primary animate-pulse shrink-0" />
        )}
        <Icon name="folder" size={16} className="text-secondary shrink-0" />
        {collapsible ? (
          <button
            type="button"
            className="flex-1 min-w-0 text-left text-sm font-medium truncate bg-transparent border-0 cursor-pointer p-0"
            onClick={() => setOpen((v) => !v)}
          >
            {title}
          </button>
        ) : (
          <span className="text-sm font-medium truncate flex-1 min-w-0">{title}</span>
        )}
        <span className="text-[10px] text-secondary shrink-0">
          {tools.length} {t("conversations.toolSteps")}
        </span>
        {collapsible && (
          <button
            type="button"
            className="dw-btn-ghost p-0.5"
            onClick={() => setOpen((v) => !v)}
            aria-expanded={open}
          >
            <Icon name={open ? "expand_less" : "expand_more"} size={18} />
          </button>
        )}
        {combinedBody && <CopyButton text={combinedBody} label={t("conversations.copyMessage")} />}
        {eventId && (
          <Link
            to="/events/$eventId"
            params={{ eventId }}
            className="dw-btn-ghost text-[10px] py-0.5 no-underline shrink-0"
          >
            <Icon name="link" size={12} />
            {t("conversations.viewEvent")}
          </Link>
        )}
      </div>
      {(!collapsible || open) && (
        <div className="divide-y divide-outline-variant/60">
          {tools.map((tool) => (
            <div key={tool.id} className="px-4 py-2.5 text-sm">
              {tool.body ? (
                <pre className="m-0 whitespace-pre-wrap break-words font-code text-xs text-on-surface">
                  {tool.body}
                </pre>
              ) : (
                <span className="text-xs text-secondary">{tool.title}</span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
