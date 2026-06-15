import { useT } from "@/i18n/context";

export function QuotaProgressBar({
  used,
  limit,
  label,
  unit,
}: {
  used: number;
  limit: number;
  label: string;
  unit?: string;
}) {
  const t = useT();
  const pct = limit > 0 ? Math.min(100, Math.round((used / limit) * 100)) : 0;
  const nearLimit = limit > 0 && used / limit >= 0.8;
  const barColor = nearLimit ? "bg-warn" : pct >= 95 ? "bg-error" : "bg-primary";

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between gap-2 text-sm">
        <span className="text-secondary">{label}</span>
        <span className="tabular-nums text-on-surface font-medium">
          {formatCount(used)}
          {unit ? ` ${unit}` : ""}
          <span className="text-secondary font-normal">
            {" "}
            / {formatCount(limit)}
            {unit ? ` ${unit}` : ""}
          </span>
        </span>
      </div>
      <div className="h-2 bg-surface-container-high rounded-full overflow-hidden">
        <div className={`h-full ${barColor} transition-all`} style={{ width: `${pct}%` }} />
      </div>
      <p className="text-xs text-secondary m-0">
        {t("service.usage.quotaUsed").replace("{pct}", String(pct))}
        {nearLimit && (
          <span className="text-warn ml-1">· {t("service.usage.quotaNearLimit")}</span>
        )}
      </p>
    </div>
  );
}

function formatCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}
