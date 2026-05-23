import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { api } from "@/api/client";
import { CopyButton } from "@/components/ui/CopyButton";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function EventDetailPage() {
  const t = useT();
  const { eventId } = useParams({ from: "/_shell/events/$eventId" });
  const event = useQuery({
    queryKey: ["event", eventId],
    queryFn: () => api.event(eventId),
  });

  if (event.isError) {
    return <div className="dw-alert-error">{(event.error as Error).message}</div>;
  }

  const e = event.data?.event;
  if (!e && !event.isLoading) {
    return <div className="dw-alert-error">{t("eventDetail.notFound")}</div>;
  }

  const hasPayload =
    e?.payload &&
    typeof e.payload === "object" &&
    Object.keys(e.payload as object).length > 0;

  return (
    <>
      <nav className="flex flex-wrap items-center gap-1 text-xs text-secondary mb-2">
        <Link to="/" className="inline-flex items-center gap-1 no-underline hover:underline">
          <Icon name="home" size={14} />
          {t("breadcrumb.home")}
        </Link>
        {e?.project_id && (
          <>
            <Icon name="chevron_right" size={14} className="text-outline" />
            <Link
              to="/projects/$projectId"
              params={{ projectId: e.project_id }}
              className="no-underline hover:underline"
            >
              {t("eventDetail.project")}
            </Link>
          </>
        )}
        {e?.session_id && (
          <>
            <Icon name="chevron_right" size={14} className="text-outline" />
            <Link
              to="/sessions/$sessionId"
              params={{ sessionId: e.session_id }}
              className="no-underline hover:underline"
            >
              {t("audit.session")}
            </Link>
          </>
        )}
      </nav>

      <PageHeader
        title={e?.title ?? eventId}
        meta={
          <>
            <span>{e?.event_type}</span>
            <span className="text-outline-variant">·</span>
            <StatusBadge status={e?.severity ?? "info"} />
            <span className="text-outline-variant">·</span>
            <span>{e?.occurred_at ?? "…"}</span>
          </>
        }
      />

      {event.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}

      {e && (
        <>
          <SectionCard title={t("eventDetail.metadata")}>
            <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
              <dt className="text-secondary font-medium m-0">{t("eventDetail.eventId")}</dt>
              <dd className="m-0 flex items-center gap-2">
                <code className="font-code">{e.id}</code>
                <CopyButton text={e.id} />
              </dd>
              <dt className="text-secondary font-medium m-0">{t("eventDetail.projectId")}</dt>
              <dd className="m-0">
                <code className="font-code">{e.project_id}</code>
              </dd>
              <dt className="text-secondary font-medium m-0">{t("eventDetail.sessionId")}</dt>
              <dd className="m-0">
                {e.session_id ? (
                  <Link
                    to="/sessions/$sessionId"
                    params={{ sessionId: e.session_id }}
                    className="font-code text-xs"
                  >
                    {e.session_id}
                  </Link>
                ) : (
                  "—"
                )}
              </dd>
              <dt className="text-secondary font-medium m-0">{t("eventDetail.taskId")}</dt>
              <dd className="m-0">
                <code className="font-code">{e.task_id ?? "—"}</code>
              </dd>
              <dt className="text-secondary font-medium m-0">{t("eventDetail.agentId")}</dt>
              <dd className="m-0">
                <code className="font-code">{e.agent_id ?? "—"}</code>
              </dd>
              <dt className="text-secondary font-medium m-0">{t("eventDetail.severity")}</dt>
              <dd className="m-0">
                <StatusBadge status={e.severity} />
              </dd>
            </dl>
          </SectionCard>

          {e.body && (
            <SectionCard
              title={t("eventDetail.body")}
              action={<CopyButton text={e.body} />}
            >
              <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-96 whitespace-pre-wrap m-0">
                {e.body}
              </pre>
            </SectionCard>
          )}

          {hasPayload && (
            <SectionCard
              title={t("eventDetail.payload")}
              action={
                <CopyButton text={JSON.stringify(e.payload, null, 2)} />
              }
            >
              <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-96 whitespace-pre-wrap m-0">
                {JSON.stringify(e.payload, null, 2)}
              </pre>
            </SectionCard>
          )}
        </>
      )}
    </>
  );
}
