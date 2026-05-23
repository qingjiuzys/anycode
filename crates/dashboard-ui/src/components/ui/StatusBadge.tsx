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

export function StatusBadge({ status }: { status: string }) {
  const t = useT();
  const key = status.toLowerCase();
  const style = STATUS_STYLE[key] ?? {
    bg: "bg-surface-variant",
    dot: "bg-outline",
    text: "text-on-surface-variant",
  };
  const labelKey = STATUS_KEYS.find((k) => k === key);
  const label = labelKey ? t(`status.${labelKey}`) : status;

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium ${style.bg} ${style.text}`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${style.dot}`} />
      {label}
    </span>
  );
}

export function TrustBar({ score }: { score: number }) {
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
