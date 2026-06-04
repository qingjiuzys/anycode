import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { ReportPreview } from "@/components/ReportPreview";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import type { ReportDocument } from "@/api/types";
import { useI18n, useT } from "@/i18n/context";

type Scope = "project" | "session";

export function ReportsPage() {
  const { locale } = useI18n();
  const t = useT();
  const params = new URLSearchParams(
    typeof window !== "undefined" ? window.location.search : "",
  );
  const [scope, setScope] = useState<Scope>(
    params.get("session_id") ? "session" : "project",
  );
  const [projectId, setProjectId] = useState(params.get("project_id") ?? "");
  const [sessionId, setSessionId] = useState(params.get("session_id") ?? "");
  const [report, setReport] = useState<ReportDocument | null>(null);

  const projects = useQuery({ queryKey: ["projects"], queryFn: () => api.projects({ limit: 500 }) });
  const sessions = useQuery({
    queryKey: ["report-sessions", projectId],
    queryFn: () => api.allSessions({ projectId, limit: 100 }),
    enabled: scope === "session" && Boolean(projectId),
  });

  const generate = useMutation({
    mutationFn: async () => {
      if (scope === "project") {
        if (!projectId) throw new Error(t("reports.selectProjectError"));
        return api.projectReport(projectId, locale);
      }
      if (!sessionId) throw new Error(t("reports.selectSessionError"));
      return api.sessionReport(sessionId, locale);
    },
    onSuccess: (data) => setReport(data.report),
  });
  const recentReports = useQuery({
    queryKey: ["recent-reports", projectId, sessionId],
    queryFn: () =>
      api.recentReports({
        projectId: projectId || undefined,
        sessionId: scope === "session" ? sessionId || undefined : undefined,
        limit: 10,
      }),
    enabled: Boolean(projectId),
  });

  useEffect(() => {
    if (params.get("project_id") && params.get("session_id")) {
      setScope("session");
    }
  }, []);

  const projectList = projects.data?.projects ?? [];
  const sessionList = sessions.data?.sessions ?? [];

  return (
    <>
      <PageHeader
        title={t("reports.title")}
        subtitle={t("reports.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("reports.title") },
        ]}
      />

      {projectList.length === 0 && !projects.isLoading && (
        <EmptyState
          title={t("reports.emptyTitle")}
          description={t("reports.emptyDesc")}
          icon="description"
        />
      )}

      {projectList.length > 0 && (
        <>
          <div className="flex flex-wrap items-center gap-2 mb-4">
            <button
              type="button"
              className={`dw-chip${scope === "project" ? " active" : ""}`}
              onClick={() => setScope("project")}
            >
              {t("reports.projectReport")}
            </button>
            <button
              type="button"
              className={`dw-chip${scope === "session" ? " active" : ""}`}
              onClick={() => setScope("session")}
            >
              {t("reports.sessionReport")}
            </button>
            <select
              className="dw-input"
              value={projectId}
              onChange={(e) => {
                setProjectId(e.target.value);
                setSessionId("");
                setReport(null);
              }}
            >
              <option value="">{t("reports.selectProject")}</option>
              {projectList.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
            {scope === "session" && (
              <select
                className="dw-input"
                value={sessionId}
                onChange={(e) => {
                  setSessionId(e.target.value);
                  setReport(null);
                }}
                disabled={!projectId}
              >
                <option value="">{t("reports.selectSession")}</option>
                {sessionList.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.title} ({s.kind})
                  </option>
                ))}
              </select>
            )}
            <button
              type="button"
              className="dw-btn-primary"
              disabled={generate.isPending || (scope === "project" ? !projectId : !sessionId)}
              onClick={() => generate.mutate()}
            >
              {generate.isPending ? t("reports.generating") : t("reports.generate")}
            </button>
          </div>

          {generate.isError && (
            <div className="dw-alert-error">{(generate.error as Error).message}</div>
          )}

          {(recentReports.data?.reports ?? []).length > 0 && (
            <SectionCard title={t("reports.recentArchived")}>
              <ul className="m-0 pl-5 text-sm space-y-1">
                {recentReports.data!.reports.map((r) => (
                  <li key={r.id}>
                    <Link to="/assets/$artifactId" params={{ artifactId: r.id }}>
                      {r.title}
                    </Link>
                    <span className="text-secondary"> · {r.updated_at}</span>
                  </li>
                ))}
              </ul>
            </SectionCard>
          )}

          {!report && !generate.isPending && (
            <EmptyState
              title={t("reports.notGenerated")}
              description={t("reports.notGeneratedDesc")}
              icon="description"
            />
          )}

          <ReportPreview report={report} loading={generate.isPending} />
        </>
      )}
    </>
  );
}
