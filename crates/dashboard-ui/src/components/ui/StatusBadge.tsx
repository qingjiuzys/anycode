import { useT } from "@/i18n/context";

const STATUS_KEYS = [
  "ok",
  "running",
  "passed",
  "verified",
  "completed",
  "warn",
  "warning",
  "unverified",
  "pending",
  "error",
  "failed",
  "blocked",
  "cancelled",
] as const;

const STATUS_STYLE: Record<string, { bg: string; dot: string; text: string }> = {
  ok: { bg: "bg-success/10", dot: "bg-success", text: "text-success" },
  running: { bg: "bg-primary/10", dot: "bg-primary-container", text: "text-primary" },
  passed: { bg: "bg-success/10", dot: "bg-success", text: "text-success" },
  verified: { bg: "bg-success/10", dot: "bg-success", text: "text-success" },
  completed: { bg: "bg-success/10", dot: "bg-success", text: "text-success" },
  warn: { bg: "bg-warn/10", dot: "bg-warn", text: "text-warn" },
  warning: { bg: "bg-warn/10", dot: "bg-warn", text: "text-warn" },
  unverified: { bg: "bg-surface-variant", dot: "bg-outline", text: "text-on-surface-variant" },
  pending: { bg: "bg-surface-variant", dot: "bg-outline", text: "text-on-surface-variant" },
  error: { bg: "bg-error/10", dot: "bg-error", text: "text-error" },
  failed: { bg: "bg-error/10", dot: "bg-error", text: "text-error" },
  blocked: { bg: "bg-error/10", dot: "bg-error", text: "text-error" },
  cancelled: { bg: "bg-surface-variant", dot: "bg-outline", text: "text-secondary" },
};

export function StatusBadge({ status, label }: { status: string; label?: string }) {
  const t = useT();
  const key = status.toLowerCase();
  const style = STATUS_STYLE[key] ?? {
    bg: "bg-surface-variant",
    dot: "bg-outline",
    text: "text-on-surface-variant",
  };
  const labelKey = STATUS_KEYS.find((k) => k === key);
  const text = label ?? (labelKey ? t(`status.${labelKey}`) : status);

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium ${style.bg} ${style.text}`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${style.dot}`} />
      {text}
    </span>
  );
}

type SessionStatusProps = {
  status: string;
  trustedStatus: string;
  pendingApprovalCount?: number;
};

/** Prefer blocked/trust over lifecycle status when both apply. Failed tasks show 失败, not 阻断. */
export function SessionStatusBadges({
  status,
  trustedStatus,
  pendingApprovalCount = 0,
}: SessionStatusProps) {
  const t = useT();
  const showFailed = status === "failed";
  const showBlocked = trustedStatus === "blocked" && !showFailed;
  const showPending = pendingApprovalCount > 0 && status === "running";
  const showRunning = status === "running" && !showBlocked && !showFailed;

  return (
    <span className="inline-flex flex-wrap items-center gap-1">
      {showFailed && <StatusBadge status="failed" />}
      {showBlocked && <StatusBadge status="blocked" />}
      {showPending && <StatusBadge status="warn" />}
      {showRunning && <StatusBadge status="running" />}
      {!showFailed && !showBlocked && !showRunning && status !== "running" && (
        <StatusBadge status={status} />
      )}
      {showBlocked && status === "running" && (
        <span className="text-[10px] text-secondary">{t("conversations.statusRunningNote")}</span>
      )}
    </span>
  );
}

export function TrustBar({ score }: { score: number | null | undefined }) {
  const t = useT();
  if (score == null) {
    return (
      <span className="text-xs text-secondary" title={t("trust.notEvaluated")}>
        —
      </span>
    );
  }
  const pct = Math.round(score * 100);
  const color =
    pct >= 90 ? "bg-success" : pct >= 70 ? "bg-warn" : pct > 0 ? "bg-error" : "bg-outline";
  return (
    <div className="flex items-center gap-2 w-24">
      <div className="flex-1 h-1.5 bg-surface-container-high rounded-full overflow-hidden">
        <div className={`h-full ${color}`} style={{ width: `${pct}%` }} />
      </div>
      <span className="text-xs text-secondary">{pct}%</span>
    </div>
  );
}
