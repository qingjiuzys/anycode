import type { DataHealth } from "@/api/types";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { translateHealthField } from "@/i18n/healthTranslate";
import { StatusBadge } from "./ui/StatusBadge";

export function DataHealthPanel({
  health,
  compact,
}: {
  health?: DataHealth;
  compact?: boolean;
}) {
  const t = useT();
  if (!health) return null;
  if (compact && health.status === "ok") return null;

  const icon =
    health.status === "error" ? "error" : health.status === "warn" ? "warning" : "check_circle";
  const iconColor =
    health.status === "error"
      ? "text-error"
      : health.status === "warn"
        ? "text-warn"
        : "text-success";

  return (
    <div
      className={`flex items-start gap-3 p-4 rounded-lg border border-outline-variant bg-surface-container-lowest shadow-sm ${compact ? "text-sm" : ""}`}
    >
      <Icon name={icon} className={iconColor} size={20} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium text-on-surface">{t("settings.dataHealth")}</span>
          <StatusBadge status={health.status} />
        </div>
        {!compact && (
          <ul className="m-0 p-0 list-none space-y-1">
            {health.checks.slice(0, 6).map((c) => (
              <li key={c.id} className="text-xs text-secondary flex gap-2">
                <StatusBadge status={c.status} />
                {translateHealthField(t, c.name)}: {translateHealthField(t, c.message)}
                {c.count > 1 && ` (${c.count})`}
              </li>
            ))}
          </ul>
        )}
        {compact && health.checks.length > 0 && (
          <p className="text-xs text-secondary m-0">
            {health.checks.length} {t("common.checks")} · {(health.db_size_bytes / 1024).toFixed(0)} KB
          </p>
        )}
      </div>
    </div>
  );
}
