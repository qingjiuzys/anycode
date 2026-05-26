import { Link } from "@tanstack/react-router";
import type { SessionReplaySummary } from "@/api/types";
import { EventTimeline } from "@/components/EventTimeline";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function SessionReplayPanel({
  replay,
  traceEventCount,
  traceSource,
}: {
  replay: SessionReplaySummary;
  traceEventCount?: number;
  traceSource?: string;
}) {
  const t = useT();

  return (
    <>
      <SectionCard title={t("session.replaySummary")}>
        {traceSource && (
          <p className="text-xs text-secondary mt-0 mb-3">
            {t("session.traceSource").replace("{source}", traceSource)}
          </p>
        )}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
          <MiniStat label={t("session.trust")} value={replay.trusted_status} badge />
          <MiniStat label={t("common.status")} value={replay.status} badge />
          <MiniStat
            label={t("session.failedGates")}
            value={String(replay.failed_gates.length)}
          />
          <MiniStat
            label={t("session.artifacts")}
            value={String(replay.artifacts.length)}
          />
          <MiniStat
            label={t("session.llmCalls")}
            value={String(replay.llm_calls_count ?? 0)}
          />
          <MiniStat
            label={t("session.toolCalls")}
            value={String(replay.tool_calls_count ?? 0)}
          />
          <MiniStat label={t("session.budget")} value={replay.budget_status ?? "ok"} badge />
          {typeof traceEventCount === "number" && (
            <MiniStat label={t("session.traceEvents")} value={String(traceEventCount)} />
          )}
        </div>

        {replay.last_error && (
          <div className="dw-alert-error mb-4">
            <span className="font-medium">{t("session.lastError")}: </span>
            <Link
              to="/events/$eventId"
              params={{ eventId: replay.last_error.event_id }}
              className="font-medium"
            >
              {replay.last_error.title}
            </Link>
            <span className="text-secondary ml-2">({replay.last_error.event_type})</span>
          </div>
        )}

        {replay.failed_gates.length > 0 && (
          <div className="mb-4">
            <h4 className="text-xs font-semibold uppercase tracking-wide m-0 mb-2">
              {t("session.failedGates")}
            </h4>
            <ul className="m-0 pl-5 text-sm space-y-1">
              {replay.failed_gates.map((g) => (
                <li key={g.id} className="flex items-center gap-2">
                  <StatusBadge status={g.status} />
                  <span>{g.name}</span>
                  {g.required && (
                    <span className="text-xs text-secondary">({t("session.required")})</span>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}

        <p className="text-sm text-secondary m-0 mb-3">
          {t("session.artifactsCount")
            .replace("{count}", String(replay.artifacts.length))
            .replace("{reports}", String(replay.report_artifacts.length))}
        </p>

        {(replay.report_artifacts ?? []).length > 0 && (
          <div className="flex flex-wrap gap-3 mb-2">
            {replay.report_artifacts.map((r) => (
              <Link
                key={r.id}
                to="/assets/$artifactId"
                params={{ artifactId: r.id }}
                className="text-sm dw-chip active no-underline"
              >
                {r.title}
              </Link>
            ))}
          </div>
        )}
      </SectionCard>

      {(replay.trace_phases ?? []).length > 0 && (
        <SectionCard title={t("session.tracePhases")}>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
            {(replay.trace_phases ?? []).map((phase) => (
              <div key={phase.phase} className="dw-stat-card">
                <div className="dw-stat-label">{t(`session.trace.${phase.phase}`)}</div>
                <div className="flex items-center justify-between gap-2">
                  <div className="dw-stat-value text-base">{phase.count}</div>
                  <StatusBadge status={phase.severity} />
                </div>
              </div>
            ))}
          </div>
        </SectionCard>
      )}

      {(replay.recent_events ?? []).length > 0 && (
        <SectionCard title={t("session.replayEvents")}>
          <EventTimeline events={replay.recent_events.slice(0, 12)} compact />
        </SectionCard>
      )}
    </>
  );
}

function MiniStat({
  label,
  value,
  badge,
}: {
  label: string;
  value: string;
  badge?: boolean;
}) {
  return (
    <div className="dw-stat-card">
      <div className="dw-stat-label">{label}</div>
      {badge ? (
        <StatusBadge status={value} />
      ) : (
        <div className="dw-stat-value text-base">{value}</div>
      )}
    </div>
  );
}
