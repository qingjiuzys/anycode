import { useEffect, useMemo, useState } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import type { TurnPhase } from "@/lib/sessionLiveStore";

const LONG_WAIT_SECONDS = 15;
const VERY_LONG_WAIT_SECONDS = 30;

type Props = {
  phase: TurnPhase;
  startedAt: string | null;
};

export function TurnPhaseBanner({ phase, startedAt }: Props) {
  const t = useT();
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!phase || !startedAt) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [phase, startedAt]);

  const elapsedSeconds = useMemo(() => {
    if (!startedAt) return 0;
    const start = Date.parse(startedAt.includes("T") ? startedAt : startedAt.replace(" ", "T"));
    if (Number.isNaN(start)) return 0;
    return Math.max(0, Math.floor((now - start) / 1000));
  }, [now, startedAt]);

  if (!phase) return null;

  const label =
    phase === "running_tools"
      ? t("conversations.turnPhaseRunningTools")
      : phase === "streaming"
        ? t("conversations.turnPhaseStreaming")
        : t("conversations.turnPhasePlanning");

  const longWait = elapsedSeconds >= LONG_WAIT_SECONDS;
  const veryLongWait = elapsedSeconds >= VERY_LONG_WAIT_SECONDS;

  return (
    <div
      className={`rounded-2xl rounded-bl-md border px-4 py-3 text-sm ${
        veryLongWait
          ? "border-warn/40 bg-warn/10"
          : "border-outline-variant/80 bg-surface-container-low"
      }`}
    >
      <div className="flex items-center gap-2 text-secondary">
        <Icon
          name={veryLongWait ? "hourglass_empty" : "progress_activity"}
          size={16}
          className={veryLongWait ? "text-warn" : "text-primary animate-spin"}
        />
        <span className="font-medium text-on-surface">{label}</span>
        <span className="text-xs tabular-nums">{elapsedSeconds}s</span>
      </div>
      {longWait && (
        <p className={`text-xs m-0 mt-1.5 ${veryLongWait ? "text-warn" : "text-secondary"}`}>
          {veryLongWait
            ? t("conversations.turnPhaseTakingLonger")
            : t("conversations.turnPhaseStillWorking")}
        </p>
      )}
    </div>
  );
}
