import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { SessionDetail, SessionWithProject } from "@/api/types";
import { CancelSessionButton } from "@/components/CancelSessionButton";
import { ConversationThread } from "@/components/ConversationThread";
import { SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { EventTimeline } from "@/components/EventTimeline";
import { GateStatusBar } from "@/components/GateStatusBar";
import { GoalRunPanel } from "@/components/GoalRunPanel";
import { TrustCompletenessPanel } from "@/components/TrustCompletenessPanel";
import { Icon } from "@/components/Icon";
import { SessionTokenUsage } from "@/components/SessionTokenUsage";
import { SessionBackgroundTasksPanel } from "@/components/SessionBackgroundTasksPanel";
import { SessionExecutionLogPanel } from "@/components/SessionExecutionLogPanel";
import { SessionReplayPanel } from "@/components/SessionReplayPanel";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useSessionEventStream } from "@/hooks/useSessionEventStream";
import { useT } from "@/i18n/context";

const SEVERITIES = ["info", "warn", "error"] as const;
const TOOL_CALL_FILTER = "tool_call_end";
type SessionTab = "chat" | "debug" | "audit";

export function SessionDetailPage() {
  const t = useT();
  const { sessionId } = useParams({ from: "/_shell/sessions/$sessionId" });
  const [tab, setTab] = useState<SessionTab>("chat");
  const [eventFilter, setEventFilter] = useState<string | null>(null);
  const [severityFilter, setSeverityFilter] = useState<string | null>(null);
  const [eventSearch, setEventSearch] = useState("");
  const sseLive = useSessionEventStream(sessionId);

  const session = useQuery({
    queryKey: ["session", sessionId],
    queryFn: () => api.session(sessionId),
    refetchInterval: sseLive ? 2_000 : false,
  });
  const eventTypes = useQuery({
    queryKey: ["session-event-types", sessionId],
    queryFn: () => api.sessionEventTypes(sessionId),
  });
  const events = useQuery({
    queryKey: [
      "session-events",
      sessionId,
      eventFilter,
      severityFilter,
      eventSearch,
    ],
    queryFn: () =>
      api.sessionEvents(sessionId, {
        eventType: eventFilter ?? undefined,
        severity: severityFilter ?? undefined,
        q: eventSearch.trim() || undefined,
      }),
    refetchInterval: sseLive ? 2_000 : false,
  });
  const gates = useQuery({
    queryKey: ["session-gates", sessionId],
    queryFn: () => api.sessionGates(sessionId),
    refetchInterval: sseLive ? 3_000 : false,
  });
  const artifacts = useQuery({
    queryKey: ["session-artifacts", sessionId],
    queryFn: () => api.sessionArtifacts(sessionId),
    refetchInterval: sseLive ? 5_000 : false,
  });
  const replay = useQuery({
    queryKey: ["session-replay", sessionId],
    queryFn: () => api.sessionReplay(sessionId),
    refetchInterval: sseLive ? 5_000 : false,
  });
  const trace = useQuery({
    queryKey: ["session-trace", sessionId],
    queryFn: () => api.sessionTrace(sessionId),
    refetchInterval: sseLive ? 5_000 : false,
  });

  if (session.isError) {
    return <div className="dw-alert-error">{(session.error as Error).message}</div>;
  }

  const s = session.data?.session;
  if (!s && !session.isLoading) {
    return <div className="dw-alert-error">{t("session.notFound")}</div>;
  }

  const meta = parseMeta(s?.metadata_json);
  const blocked = (gates.data?.gates ?? []).some(
    (g) => g.required && g.status === "failed",
  );
  const types = eventTypes.data?.event_types ?? [];
  const gateById = new Map(
    (gates.data?.gates ?? []).map((g) => [g.id, g.name]),
  );

  const failureReason =
    s?.block_reason?.trim() ||
    (s?.status === "failed" ? s?.summary?.trim() : "") ||
    null;

  const sessionForThread = s ? toSessionWithProject(s) : null;

  return (
    <>
      <nav className="flex flex-wrap items-center gap-1 text-xs text-secondary mb-2">
        <Link to="/conversations" className="inline-flex items-center gap-1 no-underline hover:underline">
          <Icon name="forum" size={14} />
          {t("nav.conversations")}
        </Link>
        {s && (
          <>
            <Icon name="chevron_right" size={14} className="text-outline" />
            <Link
              to="/projects/$projectId"
              params={{ projectId: s.project_id }}
              className="no-underline hover:underline"
            >
              {s.project_name}
            </Link>
          </>
        )}
        {sseLive && s?.status === "running" && (
          <StatusBadge status="running" />
        )}
        {s && (
          <>
            <span className="text-outline-variant">·</span>
            <Link
              to="/reports"
              search={{ project_id: s.project_id, session_id: sessionId }}
              className="inline-flex items-center gap-1 no-underline hover:underline"
            >
              <Icon name="description" size={14} />
              {t("session.generateReport")}
            </Link>
          </>
        )}
      </nav>

      <PageHeader
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("nav.conversations"), to: "/conversations" },
          { label: s?.title ?? sessionId },
        ]}
        title={s?.title ?? sessionId}
        meta={
          <>
            <span>{s?.kind}</span>
            <span className="text-outline-variant">·</span>
            <span>{s?.agent_type || "—"}</span>
            <span className="text-outline-variant">·</span>
            <span>{s?.model || "—"}</span>
            <span className="text-outline-variant">·</span>
            <StatusBadge status={s?.status ?? "pending"} />
            {s?.status === "running" && (
              <>
                <span className="text-outline-variant">·</span>
                <span>{t("session.runningRefresh")}</span>
                <span className="text-outline-variant">·</span>
                <CancelSessionButton sessionId={sessionId} status={s.status} compact />
              </>
            )}
          </>
        }
      />

      <div className="flex flex-wrap gap-2 mb-4">
        {(["chat", "debug", "audit"] as const).map((id) => (
          <button
            key={id}
            type="button"
            className={`dw-chip${tab === id ? " active" : ""}`}
            onClick={() => setTab(id)}
          >
            {t(`session.tabs.${id}`)}
          </button>
        ))}
        <Link
          to="/conversations"
          search={{ session: sessionId, project: s?.project_id }}
          className="dw-btn-ghost text-xs no-underline ml-auto"
        >
          {t("session.openInConversations")}
        </Link>
      </div>

      <GateStatusBar
        gates={gates.data?.gates ?? []}
        trustedStatus={s?.trusted_status ?? ""}
        sessionStatus={s?.status ?? ""}
      />

      {(s?.status === "failed" || s?.status === "pending") && failureReason && (
        <div className="dw-alert-error">
          <p className="m-0 font-medium">{t("session.failureReason")}</p>
          <p className="m-0 mt-1 text-sm whitespace-pre-wrap">{failureReason}</p>
        </div>
      )}

      {blocked && (
        <div className="dw-alert-error">
          {t("session.gateBlocked").replace("{status}", s?.trusted_status ?? "")}
        </div>
      )}

      {tab === "chat" && sessionForThread && (
        <div className="border border-outline-variant rounded-lg overflow-hidden bg-surface-container-lowest min-h-[min(720px,calc(100vh-14rem))] flex flex-col">
          <ConversationThread session={sessionForThread} showHeader={false} />
        </div>
      )}

      {tab === "debug" && (
        <>
          <SessionTokenUsage sessionId={sessionId} />

          <SessionBackgroundTasksPanel
            sessionId={sessionId}
            live={sseLive && s?.status === "running"}
          />

          {s?.status === "running" && (
            <SecurityApprovalInbox sessionId={sessionId} hideWhenEmpty compact />
          )}

          {replay.data?.replay && (
            <>
              <GoalRunPanel
                replay={replay.data.replay}
                agentType={s?.agent_type}
                model={s?.model}
              />
              <SessionReplayPanel
                replay={replay.data.replay}
                traceEventCount={trace.data?.trace.events.length}
                traceSource={trace.data?.trace.source}
              />
              <SessionExecutionLogPanel sessionId={sessionId} />
            </>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <SectionCard title={t("session.metadata")}>
              <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
                <dt className="text-secondary font-medium m-0">{t("session.sessionId")}</dt>
                <dd className="m-0">
                  <code className="font-code">{s?.id}</code>
                </dd>
                <dt className="text-secondary font-medium m-0">{t("session.taskId")}</dt>
                <dd className="m-0">
                  <code className="font-code">{s?.task_id ?? "—"}</code>
                </dd>
                <dt className="text-secondary font-medium m-0">{t("session.trust")}</dt>
                <dd className="m-0">
                  <StatusBadge status={s?.trusted_status ?? "pending"} />
                </dd>
                <dt className="text-secondary font-medium m-0">{t("session.startedEnded")}</dt>
                <dd className="m-0">
                  {s?.started_at} → {s?.ended_at ?? "…"}
                </dd>
                {meta.correlation_id && (
                  <>
                    <dt className="text-secondary font-medium m-0">{t("session.correlationId")}</dt>
                    <dd className="m-0">
                      <code className="font-code">{meta.correlation_id}</code>
                    </dd>
                  </>
                )}
                {meta.cron_job_id && (
                  <>
                    <dt className="text-secondary font-medium m-0">{t("session.cronJob")}</dt>
                    <dd className="m-0">
                      <code className="font-code">{meta.cron_job_id}</code>
                    </dd>
                  </>
                )}
              </dl>
              {s?.prompt_preview && (
                <>
                  <h4 className="text-xs font-semibold text-on-surface uppercase tracking-wide mt-4 mb-2">
                    {t("session.promptPreview")}
                  </h4>
                  <pre className="bg-surface-container-low border border-outline-variant rounded p-3 font-code text-xs overflow-auto max-h-48 whitespace-pre-wrap m-0">
                    {s.prompt_preview}
                  </pre>
                </>
              )}
              {s?.summary && (
                <>
                  <h4 className="text-xs font-semibold text-on-surface uppercase tracking-wide mt-4 mb-2">
                    {t("session.summary")}
                  </h4>
                  <p className="text-sm text-secondary m-0">{s.summary}</p>
                </>
              )}
            </SectionCard>
          </div>

          <SectionCard
            title={t("session.events")}
            action={
              <input
                type="search"
                className="dw-input w-48"
                placeholder={t("events.searchPlaceholder")}
                value={eventSearch}
                onChange={(e) => setEventSearch(e.target.value)}
              />
            }
          >
            <div className="flex flex-wrap gap-2 mb-2">
              <button
                type="button"
                className={`dw-chip${eventFilter === null ? " active" : ""}`}
                onClick={() => setEventFilter(null)}
              >
                {t("events.allTypes")}
              </button>
              <button
                type="button"
                className={`dw-chip${eventFilter === TOOL_CALL_FILTER ? " active" : ""}`}
                onClick={() =>
                  setEventFilter((f) => (f === TOOL_CALL_FILTER ? null : TOOL_CALL_FILTER))
                }
              >
                {t("session.filterToolCalls")}
              </button>
              {types
                .filter((etype) => !etype.startsWith("tool_call"))
                .map((etype) => (
                <button
                  key={etype}
                  type="button"
                  className={`dw-chip${eventFilter === etype ? " active" : ""}`}
                  onClick={() => setEventFilter(etype)}
                >
                  {etype}
                </button>
              ))}
            </div>
            <div className="flex flex-wrap gap-2 mb-4">
              <button
                type="button"
                className={`dw-chip${severityFilter === null ? " active" : ""}`}
                onClick={() => setSeverityFilter(null)}
              >
                {t("events.allSeverities")}
              </button>
              {SEVERITIES.map((sev) => (
                <button
                  key={sev}
                  type="button"
                  className={`dw-chip${severityFilter === sev ? " active" : ""}`}
                  onClick={() => setSeverityFilter(sev)}
                >
                  {t(`status.${sev}`)}
                </button>
              ))}
            </div>
            {events.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}
            <EventTimeline events={events.data?.events ?? []} />
          </SectionCard>
        </>
      )}

      {tab === "audit" && (
        <>
          <TrustCompletenessPanel
            gates={gates.data?.gates ?? []}
            trustedStatus={s?.trusted_status ?? ""}
            sessionStatus={s?.status ?? ""}
            sessionKind={s?.kind}
            blockReason={failureReason}
            unverifiedArtifactCount={
              (artifacts.data?.artifacts ?? []).filter((a) => a.trust_level === "unverified").length
            }
          />

          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <SectionCard title={t("session.acceptanceGates")} noPadding>
              <div className="overflow-x-auto">
                <table className="dw-table">
                  <thead>
                    <tr>
                      <th>{t("common.name")}</th>
                      <th>{t("common.status")}</th>
                      <th>{t("session.required")}</th>
                      <th>{t("session.gateOutput")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(gates.data?.gates ?? []).map((g) => (
                      <tr key={g.id}>
                        <td>{g.name}</td>
                        <td>
                          <StatusBadge status={g.status} />
                        </td>
                        <td>{g.required ? t("session.yes") : t("session.no")}</td>
                        <td className="text-secondary text-xs max-w-xs">
                          {g.output_excerpt || "—"}
                        </td>
                      </tr>
                    ))}
                    {(gates.data?.gates ?? []).length === 0 && (
                      <tr>
                        <td colSpan={4} className="text-secondary text-center py-6">
                          {t("session.noGates")}
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </SectionCard>
          </div>

          {(artifacts.data?.artifacts ?? []).length > 0 && (
            <SectionCard title={t("session.outputFiles")} noPadding>
              <div className="overflow-x-auto">
                <table className="dw-table">
                  <thead>
                    <tr>
                      <th>{t("common.path")}</th>
                      <th>{t("conversations.type")}</th>
                      <th>{t("session.trust")}</th>
                      <th>{t("session.verifyGate")}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(artifacts.data?.artifacts ?? []).map((a) => (
                      <tr key={a.id}>
                        <td className="text-secondary font-code text-xs">{a.path}</td>
                        <td>{a.kind}</td>
                        <td>
                          <StatusBadge status={a.trust_level} />
                        </td>
                        <td className="text-secondary">
                          {a.verified_by_gate_name ??
                            (a.verified_by_gate_id
                              ? gateById.get(a.verified_by_gate_id) ?? a.verified_by_gate_id
                              : "—")}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </SectionCard>
          )}
        </>
      )}
    </>
  );
}

function toSessionWithProject(s: SessionDetail): SessionWithProject {
  return {
    id: s.id,
    project_id: s.project_id,
    project_name: s.project_name,
    kind: s.kind,
    task_id: s.task_id,
    title: s.title,
    status: s.status,
    trusted_status: s.trusted_status,
    agent_type: s.agent_type,
    model: s.model,
    started_at: s.started_at,
    ended_at: s.ended_at,
    block_reason: s.block_reason,
    block_kind: s.block_kind,
  };
}

function parseMeta(raw?: string): {
  correlation_id?: string;
  cron_job_id?: string;
} {
  if (!raw) return {};
  try {
    return JSON.parse(raw) as { correlation_id?: string; cron_job_id?: string };
  } catch {
    return {};
  }
}
