import { memo } from "react";
import type { ToolStep } from "@/lib/transcriptGrouping";
import { formatAgentActivityLine } from "@/lib/agentActivitySummary";
import { useT } from "@/i18n/context";

type Props = {
  steps: ToolStep[];
  /** Hide trailing duration when TurnRecapHeader owns the main timer. */
  suppressDuration?: boolean;
};

export const AgentActivityLine = memo(function AgentActivityLine({
  steps,
  suppressDuration = false,
}: Props) {
  const t = useT();
  const line = formatAgentActivityLine(steps, t, {
    includeDuration: !suppressDuration,
  });
  if (!line) {
    return null;
  }
  return (
    <p className="agent-activity-line m-0 text-xs text-secondary leading-snug" role="status">
      {line}
    </p>
  );
});
