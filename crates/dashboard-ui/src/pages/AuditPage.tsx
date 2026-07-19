import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { ListPageToolbar } from "@/components/ui/ListPageToolbar";
import { ListPaginationBar } from "@/components/ui/ListPaginationBar";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { downloadCsv } from "@/utils/exportCsv";
import { auditActionLabel } from "@/lib/auditActions";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";

const RISKS = ["", "low", "medium", "high", "critical"] as const;
const PAGE_SIZE_OPTIONS = [25, 50, 100] as const;

function riskLabel(r: string, t: (k: string) => string) {
  if (!r) return t("audit.allRisks");
  return t(`status.${r}`) || r;
}

function exportAudit(
  rows: {
    created_at: string;
    action: string;
    risk: string;
    project_id?: string | null;
    session_id?: string | null;
    actor: string;
  }[],
  t: (k: string) => string,
) {
  downloadCsv("audit-events.csv", [
    [
      t("audit.time"),
      t("audit.action"),
      t("audit.risk"),
      t("audit.project"),
      t("audit.session"),
      t("audit.actor"),
    ],
    ...rows.map((e) => [
      e.created_at,
      e.action,
      e.risk,
      e.project_id ?? "",
      e.session_id ?? "",
      e.actor,
    ]),
  ]);
}

export function AuditPage(_props: EmbeddedPageProps = {}) {
  const t = useT();
  const [projectId, setProjectId] = useState("");
  const [action, setAction] = useState("");
  const [risk, setRisk] = useState("");
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState<number>(PAGE_SIZE_OPTIONS[0]);

  useEffect(() => {
    setPage(0);
  }, [projectId, action, risk]);

  const projects = useQuery({ queryKey: ["projects"], queryFn: () => api.projects({ limit: 500 }) });
  const audit = useQuery({
    queryKey: ["audit", projectId, action, risk, page, pageSize],
    queryFn: () =>
      api.auditEvents({
        projectId: projectId || undefined,
        action: action || undefined,
        risk: risk || undefined,
        limit: pageSize,
        offset: page * pageSize,
      }),
  });

  const rows = audit.data?.events ?? [];
  const total = audit.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));

  const filterToolbar = (
    <ListPageToolbar
      left={
        <>
          <div className="relative flex-1 sm:max-w-xs min-w-0">
            <Icon
              name="search"
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-outline pointer-events-none"
            />
            <input
              type="search"
              className="dw-input dw-input--pill w-full pl-9"
              placeholder={t("audit.actionFilter")}
              value={action}
              onChange={(e) => setAction(e.target.value)}
            />
          </div>
          <select
            className="dw-input dw-input--pill h-[34px] min-w-[140px] shrink-0 pr-8"
            value={projectId}
            onChange={(e) => setProjectId(e.target.value)}
          >
            <option value="">{t("audit.allProjects")}</option>
            {(projects.data?.projects ?? []).map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
        </>
      }
      actions={
        rows.length > 0 ? (
          <button
            type="button"
            className="dw-btn-secondary dw-btn--pill"
            onClick={() => exportAudit(rows, t)}
          >
            <Icon name="download" size={16} />
            {t("audit.export")}
          </button>
        ) : undefined
      }
      extra={RISKS.map((r) => (
        <button
          key={r || "all"}
          type="button"
          className={`dw-chip${risk === r ? " active" : ""}`}
          onClick={() => setRisk(r)}
        >
          {riskLabel(r, t)}
        </button>
      ))}
    />
  );

  return (
    <CcPageShell
      header={
        <PageHeader
          title={t("audit.title")}
          subtitle={t("audit.subtitle")}
          breadcrumbs={[
            { label: t("nav.home"), to: "/" },
            { label: t("audit.title") },
          ]}
        />
      }
    >
      {audit.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}

      {!audit.isLoading && rows.length === 0 && (
        <>
          {filterToolbar}
          <EmptyState
            title={t("audit.emptyTitle")}
            description={t("audit.emptyDesc")}
            icon="policy"
          />
        </>
      )}

      {rows.length > 0 && (
        <div className="dw-section-card dw-list-card">
          <div className="dw-list-card__toolbar px-3 py-3 border-b border-outline-variant/40">
            {filterToolbar}
          </div>
          <div className="dw-list-card__scroll">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("audit.time")}</th>
                  <th>{t("audit.action")}</th>
                  <th>{t("audit.risk")}</th>
                  <th>{t("audit.project")}</th>
                  <th>{t("audit.session")}</th>
                  <th>{t("audit.actor")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((e) => (
                  <tr key={e.id}>
                    <td className="text-secondary text-xs">{e.created_at}</td>
                    <td>
                      <span className="font-medium text-sm">{auditActionLabel(e.action, t)}</span>
                      <code className="font-code text-[10px] text-secondary ml-2">{e.action}</code>
                    </td>
                    <td>
                      <StatusBadge status={e.risk} />
                    </td>
                    <td className="text-secondary font-code text-xs">{e.project_id ?? "—"}</td>
                    <td className="text-secondary font-code text-xs">{e.session_id ?? "—"}</td>
                    <td>{e.actor}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="dw-list-card__footer">
            <ListPaginationBar
              page={page}
              pageCount={pageCount}
              pageSize={pageSize}
              pageSizeOptions={[...PAGE_SIZE_OPTIONS]}
              total={total}
              pageSizeLabel={t("audit.pageSizeLabel")}
              onPageChange={setPage}
              onPageSizeChange={(n) => {
                setPageSize(n);
                setPage(0);
              }}
            />
          </div>
        </div>
      )}
    </CcPageShell>
  );
}
