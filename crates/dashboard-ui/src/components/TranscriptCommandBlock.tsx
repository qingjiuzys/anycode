import { Link } from "@tanstack/react-router";
import type { TranscriptBlock } from "@/api/types";
import { CollapsiblePanel } from "@/components/ui/CollapsiblePanel";
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
    <CollapsiblePanel
      title={block.title || t("conversations.commandBlock")}
      defaultOpen={false}
      tone="muted"
      icon="terminal"
      headerActions={
        <>
          {block.event_id && <EventLink eventId={block.event_id} />}
          <CopyButton text={command} label={t("conversations.copyMessage")} />
        </>
      }
    >
      <pre className="m-0 font-code text-xs whitespace-pre-wrap break-words text-on-surface">
        {command}
      </pre>
    </CollapsiblePanel>
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
