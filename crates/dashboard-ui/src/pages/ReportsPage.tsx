import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { ReportPreview } from "@/components/ReportPreview";
import { CopyButton } from "@/components/ui/CopyButton";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import type { ArtifactRecord, ReportDocument } from "@/api/types";
import { useI18n, useT } from "@/i18n/context";

type Scope = "project" | "session";

export function ReportsPage() {
  const { locale } = useI18n();
  const t = useT();
  const search = useSearch({ from: "/_shell/reports" });
  const [scope, setScope] = useState<Scope>(
    search.session_id ? "session" : "project",
  );
  const [projectId, setProjectId] = useState(search.project_id ?? "");
  const [sessionId, setSessionId] = useState(search.session_id ?? "");
  const [report, setReport] = useState<ReportDocument | null>(null);
  const [libraryPreviewId, setLibraryPreviewId] = useState(
    search.artifact_id ?? "",
  );

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
  const reportLibrary = useQuery({
    queryKey: ["report-library"],
    queryFn: () => api.artifacts({ kind: "report", limit: 200 }),
  });
  const libraryPreview = useQuery({
    queryKey: ["artifact", libraryPreviewId],
    queryFn: () => api.artifactDetail(libraryPreviewId),
    enabled: Boolean(libraryPreviewId),
  });

  useEffect(() => {
    if (search.project_id && search.session_id) {
      setScope("session");
    }
  }, [search.project_id, search.session_id]);

  const projectList = projects.data?.projects ?? [];
  const sessionList = sessions.data?.sessions ?? [];
  const libraryRows = reportLibrary.data?.artifacts ?? [];
  const libraryDetail = libraryPreview.data?.artifact ?? null;

  const downloadLibraryReport = async (a: ArtifactRecord) => {
    const res = await api.artifactDetail(a.id);
    const md = res.artifact.report_markdown ?? "";
    const blob = new Blob([md], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const el = document.createElement("a");
    el.href = url;
    const base = a.path.split("/").pop() || `${a.id}.md`;
    el.download = base.endsWith(".md") ? base : `${base}.md`;
    el.click();
    URL.revokeObjectURL(url);
  };

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

          <div className="mt-6">
            <h2 className="text-base font-semibold text-on-surface m-0 mb-3">
              {t("reports.library")}
            </h2>
            {libraryRows.length === 0 && !reportLibrary.isLoading && (
              <p className="text-sm text-secondary m-0">{t("reports.libraryEmpty")}</p>
            )}
            {libraryRows.length > 0 && (
              <div className="dw-section-card overflow-hidden">
                <div className="overflow-x-auto">
                  <table className="dw-table">
                    <thead>
                      <tr>
                        <th>{t("conversations.titleCol")}</th>
                        <th>{t("reports.project")}</th>
                        <th>{t("audit.session")}</th>
                        <th>{t("assets.updated")}</th>
                        <th>{t("common.actions")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {libraryRows.map((a) => (
                        <tr key={a.id}>
                          <td>
                            <div className="font-medium">{a.title}</div>
                            <div className="font-code text-xs text-secondary">{a.path}</div>
                          </td>
                          <td>
                            {a.project_id ? (
                              <Link
                                to="/projects/$projectId"
                                params={{ projectId: a.project_id }}
                                className="no-underline hover:underline"
                              >
                                {a.project_name ?? a.project_id}
                              </Link>
                            ) : (
                              (a.project_name ?? "—")
                            )}
                          </td>
                          <td>
                            {a.session_id ? (
                              <Link
                                to="/sessions/$sessionId"
                                params={{ sessionId: a.session_id }}
                                className="no-underline hover:underline"
                              >
                                {t("assets.view")}
                              </Link>
                            ) : (
                              "—"
                            )}
                          </td>
                          <td className="text-secondary text-xs">{a.updated_at ?? "—"}</td>
                          <td>
                            <div className="flex flex-wrap items-center gap-2">
                              <button
                                type="button"
                                className="dw-btn-secondary text-xs"
                                onClick={() => setLibraryPreviewId(a.id)}
                              >
                                <Icon name="visibility" size={14} />
                                {t("reports.previewTab")}
                              </button>
                              <button
                                type="button"
                                className="dw-btn-secondary text-xs"
                                onClick={() => void downloadLibraryReport(a)}
                              >
                                <Icon name="download" size={14} />
                                {t("reports.downloadMd")}
                              </button>
                              <Link
                                to="/assets/$artifactId"
                                params={{ artifactId: a.id }}
                                className="dw-btn-secondary text-xs no-underline"
                              >
                                <Icon name="open_in_new" size={14} />
                                {t("reports.open")}
                              </Link>
                              <CopyButton
                                text={a.path}
                                label={t("artifactDetail.copyPath")}
                              />
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}

            {libraryPreviewId && (
              <div className="mt-4">
                <SectionCard
                  title={libraryDetail?.artifact.title ?? t("reports.previewTab")}
                  action={
                    <button
                      type="button"
                      className="dw-btn-secondary text-xs"
                      onClick={() => setLibraryPreviewId("")}
                    >
                      {t("reports.close")}
                    </button>
                  }
                >
                  {libraryPreview.isLoading && (
                    <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
                  )}
                  {!libraryPreview.isLoading && libraryDetail?.report_markdown && (
                    <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-[480px] whitespace-pre-wrap m-0">
                      {libraryDetail.report_markdown}
                    </pre>
                  )}
                  {!libraryPreview.isLoading && !libraryDetail?.report_markdown && (
                    <p className="text-sm text-secondary m-0">
                      {t("reports.libraryNoMarkdown")}
                    </p>
                  )}
                </SectionCard>
              </div>
            )}
          </div>
        </>
      )}
    </>
  );
}
