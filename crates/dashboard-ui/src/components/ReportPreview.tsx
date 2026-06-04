import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { ReportDocument } from "@/api/types";
import { Icon } from "@/components/Icon";
import { useClipboard } from "@/hooks/useClipboard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

interface Props {
  report: ReportDocument | null;
  loading?: boolean;
}

type PreviewTab = "preview" | "html" | "markdown";

export function ReportPreview({ report, loading }: Props) {
  const t = useT();
  const { copy, copied } = useClipboard();
  const [tab, setTab] = useState<PreviewTab>("preview");
  const [showMarkdown, setShowMarkdown] = useState(false);

  const prefs = useQuery({
    queryKey: ["dashboard-preferences"],
    queryFn: api.dashboardPreferences,
  });
  const outputFormat =
    prefs.data?.preferences?.saved?.report_output_format ??
    prefs.data?.preferences?.active?.report_output_format ??
    "markdown";

  if (loading) {
    return <p className="text-sm text-secondary">{t("reports.generating")}</p>;
  }
  if (!report) {
    return null;
  }

  const highlights = report.highlights;
  const sessions = report.sessions_recent ?? [];
  const failures = report.failure_groups ?? [];
  const gates = report.gates ?? [];
  const artifacts = report.artifacts ?? [];
  const htmlBody = report.html ?? "";
  const showMdActions = outputFormat !== "html" && report.markdown.length > 0;
  const showHtmlActions =
    (outputFormat === "html" || outputFormat === "both") && htmlBody.length > 0;

  const downloadMd = () => {
    const blob = new Blob([report.markdown], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `anycode-report-${report.scope}-${report.id.slice(0, 8)}.md`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const downloadHtml = () => {
    const blob = new Blob([htmlBody], { type: "text/html" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `anycode-report-${report.scope}-${report.id.slice(0, 8)}.html`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const generationLabel = report.generation_mode
    ? `${t("reports.generationMode")}: ${report.generation_mode}`
    : null;

  return (
    <div className="dw-report-preview">
      <div className="dw-report-preview__head">
        <div className="min-w-0">
          <h2 className="text-lg font-semibold text-on-surface m-0 truncate">{report.title}</h2>
          <p className="text-xs text-secondary m-0 mt-1 flex flex-wrap items-center gap-x-2 gap-y-1">
            <span>
              {report.scope === "project" ? t("reports.projectReport") : t("reports.sessionReport")}
            </span>
            <StatusBadge status={report.trusted_status} />
            <span className="font-code">{formatTime(report.generated_at)}</span>
            {generationLabel && <span className="text-outline">{generationLabel}</span>}
          </p>
        </div>
        <div className="flex flex-wrap gap-2 shrink-0">
          {showMdActions && (
            <>
              <button
                type="button"
                className="dw-btn-secondary text-sm"
                onClick={() => copy(report.markdown)}
              >
                <Icon name="content_copy" size={16} />
                {copied ? t("common.copied") : t("reports.copyMarkdown")}
              </button>
              <button type="button" className="dw-btn-secondary text-sm" onClick={downloadMd}>
                <Icon name="download" size={16} />
                {t("reports.downloadMd")}
              </button>
            </>
          )}
          {showHtmlActions && (
            <>
              <button
                type="button"
                className="dw-btn-secondary text-sm"
                onClick={() => copy(htmlBody)}
              >
                <Icon name="content_copy" size={16} />
                {t("reports.copyHtml")}
              </button>
              <button type="button" className="dw-btn-secondary text-sm" onClick={downloadHtml}>
                <Icon name="download" size={16} />
                {t("reports.downloadHtml")}
              </button>
            </>
          )}
        </div>
      </div>

      <div className="flex flex-wrap gap-2 px-4 sm:px-5 pt-3 border-b border-outline-variant/30">
        {(["preview", "html", "markdown"] as PreviewTab[]).map((id) => {
          if (id === "html" && !htmlBody) return null;
          if (id === "markdown" && !report.markdown) return null;
          const label =
            id === "preview"
              ? t("reports.previewTab")
              : id === "html"
                ? t("reports.htmlTab")
                : t("reports.markdownTab");
          return (
            <button
              key={id}
              type="button"
              className={`dw-chip${tab === id ? " active" : ""}`}
              onClick={() => setTab(id)}
            >
              {label}
            </button>
          );
        })}
      </div>

      {tab === "html" && htmlBody && (
        <div className="px-4 sm:px-5 py-4">
          <iframe
            title="report-html"
            className="w-full min-h-[480px] rounded-lg border border-outline-variant/50 bg-white"
            sandbox=""
            srcDoc={htmlBody}
          />
        </div>
      )}

      {tab === "markdown" && report.markdown && (
        <div className="dw-report-markdown-fold px-4 sm:px-5 py-4">
          <pre className="dw-report-markdown-pre m-0">{report.markdown}</pre>
        </div>
      )}

      {tab === "preview" && (
        <>
          {highlights && (
            <div className="dw-report-verdict">
              <p className="dw-report-verdict__text m-0">{highlights.verdict}</p>
              <div className="dw-report-kpi-row">
                <Kpi label={t("reports.trustVerified")} value={highlights.trust_verified} />
                <Kpi label={t("reports.trustUnverified")} value={highlights.trust_unverified} />
                <Kpi label={t("reports.trustBlocked")} value={highlights.trust_blocked} />
                <Kpi label={t("reports.failureTypes")} value={highlights.failures_unique} />
              </div>
            </div>
          )}

          <div className="dw-report-section">
            <h3 className="dw-report-section__title">{t("reports.sectionSessions")}</h3>
            {sessions.length === 0 && !(report.sessions_imported_count ?? 0) ? (
              <p className="text-sm text-secondary m-0">{t("reports.noSessions")}</p>
            ) : (
              <>
                <div className="overflow-x-auto rounded-lg border border-outline-variant/40">
                  <table className="dw-table text-sm">
                    <thead>
                      <tr>
                        <th>{t("conversations.titleCol")}</th>
                        <th>{t("conversations.type")}</th>
                        <th>{t("common.status")}</th>
                        <th>{t("conversations.trust")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {sessions.map((row) => (
                        <tr key={row.session_id}>
                          <td>
                            <Link
                              to="/sessions/$sessionId"
                              params={{ sessionId: row.session_id }}
                              className="font-medium no-underline hover:underline"
                            >
                              {row.title}
                            </Link>
                          </td>
                          <td className="text-secondary text-xs">{row.kind}</td>
                          <td>
                            <StatusBadge status={row.status} />
                          </td>
                          <td>
                            <StatusBadge status={row.trusted_status} />
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {(report.sessions_imported_count ?? 0) > 0 && (
                  <p className="text-xs text-secondary m-0 mt-2">
                    {t("reports.importedCollapsed").replace(
                      "{n}",
                      String(report.sessions_imported_count),
                    )}
                  </p>
                )}
              </>
            )}
          </div>

          {failures.length > 0 && (
            <div className="dw-report-section">
              <h3 className="dw-report-section__title">{t("reports.sectionFailures")}</h3>
              <ul className="dw-report-failure-list m-0 p-0 list-none">
                {failures.map((g) => (
                  <li key={`${g.title}-${g.event_type}`} className="dw-report-failure-item">
                    <div className="min-w-0 flex-1">
                      <span className="font-medium text-sm">{g.title}</span>
                      <span className="text-xs text-secondary ml-2 font-code">{g.event_type}</span>
                    </div>
                    <span className="text-xs text-secondary tabular-nums shrink-0">×{g.count}</span>
                    <span className="text-xs text-secondary font-code shrink-0">
                      {formatTime(g.last_at)}
                    </span>
                    {g.session_id && (
                      <Link
                        to="/sessions/$sessionId"
                        params={{ sessionId: g.session_id }}
                        className="text-xs text-primary no-underline shrink-0"
                      >
                        {t("reports.viewSession")}
                      </Link>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {gates.length > 0 && (
            <div className="dw-report-section">
              <h3 className="dw-report-section__title">{t("reports.sectionGates")}</h3>
              <ul className="m-0 p-0 list-none space-y-2 text-sm">
                {gates.map((g) => (
                  <li key={g.name} className="rounded-lg bg-surface-container-low px-3 py-2">
                    <span className="font-medium">{g.name}</span>
                    <StatusBadge status={g.status} />
                    <span className="text-xs text-secondary ml-2">
                      {g.required ? t("reports.gateRequired") : t("reports.gateOptional")}
                    </span>
                    {g.output_excerpt && (
                      <p className="text-xs text-secondary m-0 mt-1 font-code line-clamp-2">
                        {g.output_excerpt}
                      </p>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {artifacts.length > 0 && (
            <div className="dw-report-section">
              <h3 className="dw-report-section__title">{t("reports.sectionArtifacts")}</h3>
              <ul className="m-0 p-0 list-none space-y-1 text-sm font-code text-secondary">
                {artifacts.map((a) => (
                  <li key={a.path}>
                    {a.path} · {a.kind} · {a.trust_level}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <div className="dw-report-meta">
            {report.root_path && (
              <span className="font-code text-xs text-secondary truncate" title={report.root_path}>
                {report.root_path}
              </span>
            )}
            {report.project_id && (
              <span className="font-code text-[11px] text-outline">ID: {report.project_id}</span>
            )}
            {report.events_sample_limit != null && report.events_sample_limit > 0 && (
              <span className="text-xs text-secondary">
                {t("reports.eventsSampleNote").replace("{n}", String(report.events_sample_limit))}
              </span>
            )}
          </div>

          {report.markdown && (
            <div className="dw-report-markdown-fold">
              <button
                type="button"
                className="dw-btn-ghost text-sm w-full justify-between"
                onClick={() => setShowMarkdown((v) => !v)}
              >
                <span className="inline-flex items-center gap-1">
                  <Icon name="description" size={16} />
                  {t("reports.markdownExport")}
                </span>
                <Icon name={showMarkdown ? "expand_less" : "expand_more"} size={18} />
              </button>
              {showMarkdown && (
                <pre className="dw-report-markdown-pre m-0 mt-2">{report.markdown}</pre>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}

function Kpi({ label, value }: { label: string; value: number }) {
  return (
    <div className="dw-report-kpi">
      <div className="dw-report-kpi__value tabular-nums">{value}</div>
      <div className="dw-report-kpi__label">{label}</div>
    </div>
  );
}

function formatTime(raw: string): string {
  const n = raw.replace("T", " ");
  return n.length > 19 ? n.slice(0, 19) : n;
}
