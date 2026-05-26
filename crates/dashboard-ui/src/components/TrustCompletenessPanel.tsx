import type { GateRecord } from "@/api/types";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

interface Props {
  gates: GateRecord[];
  trustedStatus: string;
  sessionStatus: string;
  sessionKind?: string;
  blockReason?: string | null;
  unverifiedArtifactCount?: number;
}

export function TrustCompletenessPanel({
  gates,
  trustedStatus,
  sessionStatus,
  sessionKind,
  blockReason,
  unverifiedArtifactCount = 0,
}: Props) {
  const t = useT();
  const required = gates.filter((g) => g.required);
  const failed = required.filter((g) => g.status === "failed");
  const pending = required.filter((g) => g.status === "pending" || g.status === "running");
  const passed = required.filter((g) => g.status === "passed");

  if (required.length === 0 && trustedStatus === "verified") {
    return null;
  }

  const complete = trustedStatus === "verified" && failed.length === 0 && pending.length === 0;
  const noGatesKind = sessionKind === "run" || sessionKind === "repl" || sessionKind === "cron";

  return (
    <SectionCard title={t("trust.title")}>
      {complete ? (
        <p className="text-sm text-success m-0 flex items-center gap-2">
          <Icon name="verified" size={18} />
          {t("trust.complete")}
        </p>
      ) : (
        <ul className="m-0 p-0 list-none space-y-2 text-sm">
          {sessionStatus === "failed" && blockReason && (
            <CheckItem ok={false} label={t("trust.taskFailed")} detail={blockReason} />
          )}
          {sessionStatus === "running" && (
            <CheckItem ok={false} label={t("trust.sessionRunning")} />
          )}
          {failed.map((g) => (
            <CheckItem
              key={g.id}
              ok={false}
              label={t("trust.gateFailed").replace("{name}", g.name)}
              detail={g.output_excerpt || undefined}
              badge={g.status}
            />
          ))}
          {pending.map((g) => (
            <CheckItem
              key={g.id}
              ok={false}
              pending
              label={t("trust.gatePending").replace("{name}", g.name)}
              badge={g.status}
            />
          ))}
          {unverifiedArtifactCount > 0 && (
            <CheckItem
              ok={false}
              label={t("trust.unverifiedAssets").replace("{n}", String(unverifiedArtifactCount))}
            />
          )}
          {trustedStatus === "blocked" && failed.length === 0 && sessionStatus !== "failed" && (
            <CheckItem ok={false} label={t("trust.blockedNoGate")} />
          )}
          {trustedStatus === "unverified" && failed.length === 0 && pending.length === 0 && (
            sessionStatus === "running" && noGatesKind && required.length === 0 ? (
              <CheckItem ok={false} label={t("trust.noGatesConfigured")} />
            ) : (
              <CheckItem ok={false} label={t("trust.unverifiedManual")} />
            )
          )}
          {passed.length > 0 && failed.length === 0 && pending.length === 0 && trustedStatus !== "verified" && (
            <CheckItem ok={false} label={t("trust.awaitingReview")} />
          )}
        </ul>
      )}
    </SectionCard>
  );
}

function CheckItem({
  ok,
  pending,
  label,
  detail,
  badge,
}: {
  ok: boolean;
  pending?: boolean;
  label: string;
  detail?: string;
  badge?: string;
}) {
  const icon = ok ? "check_circle" : pending ? "schedule" : "cancel";
  const color = ok ? "text-success" : pending ? "text-warn" : "text-error";
  return (
    <li className="flex gap-2 items-start">
      <Icon name={icon} size={18} className={`shrink-0 mt-0.5 ${color}`} />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span>{label}</span>
          {badge && <StatusBadge status={badge} />}
        </div>
        {detail && (
          <pre className="text-xs text-secondary mt-1 mb-0 whitespace-pre-wrap font-code bg-surface-container-low rounded p-2 max-h-24 overflow-auto">
            {detail}
          </pre>
        )}
      </div>
    </li>
  );
}
