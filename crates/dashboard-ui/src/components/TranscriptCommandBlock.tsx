import { Link } from "@tanstack/react-router";
import type { TranscriptBlock } from "@/api/types";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  block: TranscriptBlock;
};

export function TranscriptCommandBlock({ block }: Props) {
  const t = useT();
  const command = extractCommand(block.body, block.title);

  return (
    <div className="rounded-xl border border-outline-variant bg-surface-container-highest overflow-hidden">
      <div className="flex items-center justify-between gap-2 px-3 py-2 border-b border-outline-variant/70 bg-surface-container-low">
        <span className="inline-flex items-center gap-2 text-xs font-medium text-secondary min-w-0">
          <span className="font-code text-primary">&gt;_</span>
          <span className="truncate">{block.title || t("conversations.commandBlock")}</span>
        </span>
        <div className="flex items-center gap-1 shrink-0">
          {block.event_id && <EventLink eventId={block.event_id} />}
          <CopyButton text={command} label={t("conversations.copyMessage")} />
        </div>
      </div>
      <pre className="m-0 px-3 py-2.5 font-code text-xs whitespace-pre-wrap break-words text-on-surface">
        {command}
      </pre>
    </div>
  );
}

function extractCommand(body: string, title: string): string {
  const trimmed = body.trim();
  if (trimmed) return trimmed;
  return title.trim();
}

function EventLink({ eventId }: { eventId: string }) {
  const t = useT();
  return (
    <Link
      to="/events/$eventId"
      params={{ eventId }}
      className="dw-btn-ghost text-[10px] py-0.5 no-underline"
    >
      <Icon name="link" size={12} />
      {t("conversations.viewEvent")}
    </Link>
  );
}

export function isCommandBlock(block: TranscriptBlock): boolean {
  const type = block.block_type.toLowerCase();
  if (type.includes("command") || type.includes("shell") || type.includes("bash")) {
    return true;
  }
  const title = block.title.toLowerCase();
  return (
    title.includes("bash") ||
    title.includes("shell") ||
    title.includes("command") ||
    title.startsWith("$ ")
  );
}
