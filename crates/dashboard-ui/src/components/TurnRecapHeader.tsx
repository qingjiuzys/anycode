import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import {
  formatAgentActivityRecap,
  formatAgentActivityLine,
} from "@/lib/agentActivitySummary";
import type { TurnPhase } from "@/lib/sessionLiveStore";
import type { ToolStep } from "@/lib/transcriptGrouping";
import {
  LONG_WAIT_SECONDS,
  VERY_LONG_WAIT_SECONDS,
} from "@/lib/turnLiveStatus";
import { formatDuration } from "@/utils/formatTime";

type Props = {
  turnStartedAt: string;
  turnEndedAt?: string | null;
  isRunning: boolean;
  phase?: TurnPhase | null;
  toolSteps?: ToolStep[];
  sessionId?: string;
  lastUserPrompt?: string | null;
  stallSeconds?: number;
  showStallActions?: boolean;
  compact?: boolean;
  waitingForUser?: boolean;
  latestProgressSummary?: string | null;
};

const PHASE_CHIP: Record<
  NonNullable<TurnPhase>,
  { labelKey: string; icon: string }
> = {
  waiting_first_token: {
    labelKey: "conversations.turnPhasePlanning",
    icon: "psychology",
  },
  streaming: {
    labelKey: "conversations.turnPhaseStreaming",
    icon: "edit",
  },
  running_tools: {
    labelKey: "conversations.turnPhaseRunningTools",
    icon: "build",
  },
};

export function TurnRecapHeader({
  turnStartedAt,
  turnEndedAt = null,
  isRunning,
  phase = null,
  toolSteps = [],
  sessionId,
  lastUserPrompt = null,
  stallSeconds = 0,
  showStallActions = false,
  compact = false,
  waitingForUser = false,
  latestProgressSummary = null,
}: Props) {
  const t = useT();
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!isRunning) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [isRunning]);

  const durationLabel = useMemo(() => {
    if (isRunning) {
      return formatDuration(turnStartedAt, new Date(now).toISOString());
    }
    if (turnEndedAt) {
      return formatDuration(turnStartedAt, turnEndedAt);
    }
    return formatDuration(turnStartedAt);
  }, [isRunning, now, turnEndedAt, turnStartedAt]);

  const headerTitle =
    latestProgressSummary?.trim() ||
    (isRunning
      ? t("conversations.turnRecapProcessing").replace("{duration}", durationLabel)
      : t("conversations.turnRecapWorked").replace("{duration}", durationLabel));

  const showDurationMeta = Boolean(latestProgressSummary?.trim());

  const activityLine =
    !compact && !waitingForUser && !latestProgressSummary
      ? isRunning
        ? formatAgentActivityLine(toolSteps, t, { includeDuration: false })
        : formatAgentActivityRecap(toolSteps, t)
      : null;

  const longWait =
    isRunning && !waitingForUser && stallSeconds >= LONG_WAIT_SECONDS;
  const veryLongWait =
    isRunning && !waitingForUser && stallSeconds >= VERY_LONG_WAIT_SECONDS;
  const showWarn = veryLongWait || (longWait && Boolean(phase));

  const queryClient = useQueryClient();
  const invalidate = () => {
    if (!sessionId) return;
    void queryClient.invalidateQueries({ queryKey: ["session-transcript", sessionId] });
    void queryClient.invalidateQueries({ queryKey: ["session", sessionId] });
    void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
  };
  const cancel = useMutation({
    mutationFn: () => api.cancelSession(sessionId!),
    onSuccess: invalidate,
  });
  const retry = useMutation({
    mutationFn: async () => {
      await api.cancelSession(sessionId!).catch(() => undefined);
      if (lastUserPrompt) {
        await api.sendSessionMessage(sessionId!, { prompt: lastUserPrompt });
      }
    },
    onSuccess: invalidate,
  });

  return (
    <div
      className={`turn-recap-header rounded-xl border px-3 py-2 text-sm ${
        showWarn
          ? "border-warn/35 bg-warn/5"
          : "border-outline-variant/60 bg-surface-container-low/80"
      }`}
      data-testid="turn-recap-header"
    >
      <div className="flex flex-wrap items-center gap-2 min-w-0 turn-recap-header__title-row">
        {isRunning ? (
          <Icon
            name={veryLongWait ? "hourglass_empty" : "progress_activity"}
            size={15}
            className={`shrink-0 ${veryLongWait ? "text-warn" : "text-primary animate-spin"}`}
          />
        ) : (
          <Icon name="schedule" size={15} className="shrink-0 text-secondary" />
        )}
        <span className="font-medium text-on-surface tabular-nums min-w-0 truncate">{headerTitle}</span>
        {showDurationMeta && (
          <span className="text-xs text-secondary tabular-nums shrink-0">{durationLabel}</span>
        )}
        {waitingForUser && (
          <span className="turn-phase-chip inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] border border-primary/30 bg-primary/10 text-primary">
            <Icon name="quiz" size={12} />
            <span>{t("conversations.turnWaitingForUser")}</span>
          </span>
        )}
        {isRunning && phase && !waitingForUser && (
          <span className="turn-recap-header__phase-slot shrink-0">
            <TurnPhaseChip phase={phase} done={false} />
          </span>
        )}
        {!isRunning && (
          <span className="turn-phase-chip turn-phase-chip--done text-[10px]">
            {t("conversations.turnPhaseDone")}
          </span>
        )}
      </div>

      {activityLine && (
        <p className="agent-activity-line m-0 mt-1 text-xs text-secondary leading-snug">
          {activityLine}
        </p>
      )}

      {longWait && isRunning && (
        <p className={`text-xs m-0 mt-1.5 ${veryLongWait ? "text-warn" : "text-secondary"}`}>
          {veryLongWait
            ? t("conversations.turnPhaseTakingLonger")
            : t("conversations.turnPhaseStillWorking")}
        </p>
      )}

      {veryLongWait && showStallActions && sessionId && (
        <div className="flex items-center gap-2 mt-2">
          <button
            type="button"
            className="dw-btn-secondary text-xs"
            disabled={cancel.isPending}
            onClick={() => cancel.mutate()}
          >
            {t("conversations.stalledCancel")}
          </button>
          {lastUserPrompt && (
            <button
              type="button"
              className="dw-btn-secondary text-xs"
              disabled={retry.isPending}
              onClick={() => retry.mutate()}
            >
              {t("conversations.stalledRetry")}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

function TurnPhaseChip({ phase, done }: { phase: TurnPhase; done: boolean }) {
  const t = useT();
  if (!phase) return null;
  const spec = PHASE_CHIP[phase];
  return (
    <span
      className={`turn-phase-chip inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] border ${
        done
          ? "turn-phase-chip--done"
          : "border-primary/30 bg-primary/10 text-primary"
      }`}
      data-testid="turn-phase-chip"
    >
      <Icon name={spec.icon} size={12} />
      <span>{t(spec.labelKey)}</span>
    </span>
  );
}
