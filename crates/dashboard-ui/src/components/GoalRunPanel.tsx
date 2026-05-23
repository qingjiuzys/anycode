import { Link } from "@tanstack/react-router";
import type { SessionReplaySummary, ToolCallSummary } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

interface Props {
  replay: SessionReplaySummary;
  agentType?: string;
  model?: string;
}

export function GoalRunPanel({ replay, agentType, model }: Props) {
  const t = useT();
  const toolCalls = replay.tool_calls_recent ?? [];
  const attempts = replay.attempt_count ?? 0;
  const isGoalLike =
    replay.kind === "goal" ||
    replay.kind === "workflow" ||
    replay.status === "running" ||
    replay.kind === "run" ||
    replay.kind === "repl";

  if (!isGoalLike && toolCalls.length === 0 && attempts === 0 && !replay.last_error) {
    return null;
  }

  return (
    <SectionCard title={t("goalRun.title")}>
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 mb-4">
        <Mini label={t("goalRun.attempts")} value={String(attempts || "—")} />
        <Mini
          label={t("goalRun.activeAgent")}
          value={replay.active_agent || agentType || "—"}
        />
        <Mini label={t("goalRun.model")} value={model || "—"} />
        <Mini label={t("common.status")} value={<StatusBadge status={replay.status} />} />
      </div>
      {(replay.kind === "goal" || replay.kind === "workflow") && (
        <p className="text-xs text-secondary m-0 mb-3">{t("goalRun.attemptsHint")}</p>
      )}

      {replay.last_error && (
        <div className="mb-4 p-3 rounded-lg bg-error-container text-on-error-container text-sm">
          <div className="font-semibold mb-1">{t("goalRun.lastError")}</div>
          <div className="font-medium">{replay.last_error.title}</div>
          {replay.last_error.body && (
            <pre className="mt-2 m-0 whitespace-pre-wrap text-xs opacity-90">{replay.last_error.body}</pre>
          )}
          <Link
            to="/events/$eventId"
            params={{ eventId: replay.last_error.event_id }}
            className="text-xs text-primary mt-2 inline-block"
          >
            {t("common.details")}
          </Link>
        </div>
      )}

      {toolCalls.length > 0 && (
        <>
          <h4 className="text-xs font-semibold uppercase tracking-wide m-0 mb-2">
            {t("goalRun.recentTools")}
          </h4>
          <div className="overflow-x-auto -mx-1">
            <table className="dw-table text-sm">
              <thead>
                <tr>
                  <th>{t("goalRun.tool")}</th>
                  <th>{t("common.status")}</th>
                  <th>{t("audit.time")}</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {toolCalls.map((tc) => (
                  <ToolRow key={tc.event_id} tc={tc} />
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}

      {toolCalls.length === 0 && replay.status === "running" && (
        <p className="text-sm text-secondary m-0">{t("goalRun.noToolsYet")}</p>
      )}
    </SectionCard>
  );
}

function ToolRow({ tc }: { tc: ToolCallSummary }) {
  const t = useT();
  const failed = tc.event_type.includes("fail") || tc.title.toLowerCase().includes("error");
  return (
    <tr>
      <td className="font-medium">{tc.tool_name || tc.title}</td>
      <td>
        <StatusBadge status={failed ? "failed" : "passed"} />
      </td>
      <td className="text-secondary text-xs">{tc.occurred_at}</td>
      <td>
        <Link
          to="/events/$eventId"
          params={{ eventId: tc.event_id }}
          className="text-xs text-primary no-underline hover:underline"
        >
          {t("common.details")}
        </Link>
      </td>
    </tr>
  );
}

function Mini({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="dw-stat-card">
      <div className="dw-stat-label">{label}</div>
      <div className="dw-stat-value text-sm">{value}</div>
    </div>
  );
}
