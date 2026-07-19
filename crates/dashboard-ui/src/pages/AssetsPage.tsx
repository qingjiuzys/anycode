import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { api } from "@/api/client";
import type { AssetItem } from "@/api/types/artifacts";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { ListPageToolbar } from "@/components/ui/ListPageToolbar";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { downloadCsv } from "@/utils/exportCsv";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";
import { sessionChatSearch } from "@/lib/sessionLinks";

type TrustFilter = "all" | "unverified" | "blocked";
type AssetKindFilter =
  | ""
  | "all"
  | "deliverable"
  | "media"
  | "report"
  | "workflow"
  | "skill";
type SourceFilter = "" | "agent_created" | "workspace_scan" | "report_archive" | "skill_scan" | "workflow_scan";
type ReuseFilter = "" | "candidate" | "reusable" | "archived";

export function AssetsPage({ embedded, initialSearch }: EmbeddedPageProps = {}) {
  if (embedded) {
    const trust = initialSearch?.trust;
    return (
      <AssetsPageInner
        trustSearch={trust === "unverified" || trust === "blocked" ? trust : undefined}
      />
    );
  }
  return <AssetsPageRouted />;
}

function AssetsPageRouted() {
  const { trust: trustSearch } = useSearch({ from: "/_shell/assets" });
  return <AssetsPageInner trustSearch={trustSearch} />;
}

function AssetsPageInner({
  trustSearch,
}: {
  trustSearch?: "unverified" | "blocked";
}) {
  const t = useT();
  const queryClient = useQueryClient();
  const projects = useQuery({
    queryKey: ["projects"],
    queryFn: () => api.projects({ limit: 500 }),
  });
  const list = projects.data?.projects ?? [];
  const [projectId, setProjectId] = useState<string>("");
  const [assetKind, setAssetKind] = useState<AssetKindFilter>("");
  const [sourceType, setSourceType] = useState<SourceFilter>("");
  const [reuseState, setReuseState] = useState<ReuseFilter>("");
  const [trustFilter, setTrustFilter] = useState<TrustFilter>(
    trustSearch ?? "all",
  );
  const [reportPreviewId, setReportPreviewId] = useState("");

  useEffect(() => {
    if (trustSearch) setTrustFilter(trustSearch);
  }, [trustSearch]);

  useEffect(() => {
    setReportPreviewId("");
  }, [projectId, assetKind]);

  const reportPreview = useQuery({
    queryKey: ["artifact", reportPreviewId],
    queryFn: () => api.artifactDetail(reportPreviewId),
    enabled: Boolean(reportPreviewId),
  });

  const assets = useQuery({
    queryKey: [
      "assets",
      projectId,
      assetKind,
      sourceType,
      reuseState,
      trustFilter,
    ],
    queryFn: () =>
      api.assets({
        projectId: projectId || undefined,
        assetKind: assetKind && assetKind !== "all" ? assetKind : assetKind === "all" ? "all" : "deliverable",
        sourceType: sourceType || undefined,
        reuseState: reuseState || undefined,
        unverifiedOnly: trustFilter === "unverified",
        blockedSessionOnly: trustFilter === "blocked",
        finalOnly: assetKind === "",
        includeSkills: assetKind === "" || assetKind === "all" || assetKind === "skill",
        limit: 200,
      }),
  });

  const reindex = useMutation({
    mutationFn: async () => {
      if (!projectId) return;
      await api.scanProjectWorkflows(projectId);
      await api.indexProjectAssets(projectId);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["assets"] });
    },
  });

  const rows = assets.data?.assets ?? [];

  const trustFilters = [
    { id: "all" as const, label: t("assets.filterAll") },
    { id: "unverified" as const, label: t("assets.filterUnverified") },
    { id: "blocked" as const, label: t("assets.filterBlocked") },
  ];

  const kindOptions: { value: AssetKindFilter; label: string }[] = [
    { value: "", label: t("assets.deliverables") },
    { value: "all", label: t("assets.allTypes") },
    { value: "deliverable", label: t("assets.kinds.deliverable") },
    { value: "media", label: t("assets.kinds.media") },
    { value: "report", label: t("assets.kinds.report") },
    { value: "workflow", label: t("assets.kinds.workflow") },
    { value: "skill", label: t("assets.kinds.skill") },
  ];

  const sourceOptions: { value: SourceFilter; label: string }[] = [
    { value: "", label: t("assets.allSources") },
    { value: "agent_created", label: t("assets.sources.agent_created") },
    { value: "workspace_scan", label: t("assets.sources.workspace_scan") },
    { value: "report_archive", label: t("assets.sources.report_archive") },
    { value: "skill_scan", label: t("assets.sources.skill_scan") },
    { value: "workflow_scan", label: t("assets.sources.workflow_scan") },
  ];

  const reuseOptions: { value: ReuseFilter; label: string }[] = [
    { value: "", label: t("assets.allReuseStates") },
    { value: "candidate", label: t("assets.reuseStates.candidate") },
    { value: "reusable", label: t("assets.reuseStates.reusable") },
    { value: "archived", label: t("assets.reuseStates.archived") },
  ];

  const filterToolbar = (
    <ListPageToolbar
      left={
        <>
          <select
            className="dw-input dw-input--pill h-[34px] min-w-[120px] shrink-0 pr-8"
            value={projectId}
            onChange={(e) => setProjectId(e.target.value)}
          >
            <option value="">{t("assets.allProjects")}</option>
            {list.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <select
            className="dw-input dw-input--pill h-[34px] min-w-[120px] shrink-0 pr-8"
            value={assetKind}
            onChange={(e) => setAssetKind(e.target.value as AssetKindFilter)}
          >
            {kindOptions.map((o) => (
              <option key={o.value || "default"} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          <select
            className="dw-input dw-input--pill h-[34px] min-w-[120px] shrink-0 pr-8"
            value={sourceType}
            onChange={(e) => setSourceType(e.target.value as SourceFilter)}
          >
            {sourceOptions.map((o) => (
              <option key={o.value || "all-sources"} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          <select
            className="dw-input dw-input--pill h-[34px] min-w-[120px] shrink-0 pr-8"
            value={reuseState}
            onChange={(e) => setReuseState(e.target.value as ReuseFilter)}
          >
            {reuseOptions.map((o) => (
              <option key={o.value || "all-reuse"} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </>
      }
      actions={
        <>
          <ControlCenterLink
            to="/reports"
            search={projectId ? { project_id: projectId } : undefined}
            className="dw-btn-secondary dw-btn--pill no-underline inline-flex items-center gap-1.5"
          >
            <Icon name="description" size={16} />
            {t("assets.openReportGenerator")}
          </ControlCenterLink>
          {projectId && (
            <button
              type="button"
              className="dw-btn-secondary dw-btn--pill"
              disabled={reindex.isPending}
              onClick={() => reindex.mutate()}
            >
              <Icon name="sync" size={16} />
              {t("assets.reindexProject")}
            </button>
          )}
          {rows.length > 0 && (
            <button
              type="button"
              className="dw-btn-secondary dw-btn--pill"
              onClick={() => exportAssets(rows, t)}
            >
              <Icon name="download" size={16} />
              {t("assets.export")}
            </button>
          )}
        </>
      }
      extra={trustFilters.map((f) => (
        <button
          key={f.id}
          type="button"
          className={`dw-chip${trustFilter === f.id ? " active" : ""}`}
          onClick={() => setTrustFilter(f.id)}
        >
          {f.label}
        </button>
      ))}
    />
  );

  return (
    <CcPageShell
      header={
        <PageHeader
          title={t("assets.title")}
          subtitle={t("assets.subtitle")}
          breadcrumbs={[
            { label: t("nav.home"), to: "/" },
            { label: t("assets.title") },
          ]}
        />
      }
    >
      {assets.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}

      {!assets.isLoading && rows.length === 0 && (
        <>
          {filterToolbar}
          <EmptyState
            title={t("assets.emptyTitle")}
            description={t("assets.emptyDesc")}
            icon="inventory_2"
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
                  <th>{t("assets.project")}</th>
                  <th>{t("assets.name")}</th>
                  <th>{t("assets.type")}</th>
                  <th>{t("assets.source")}</th>
                  <th>{t("assets.reuseState")}</th>
                  <th>{t("audit.session")}</th>
                  <th>{t("conversations.trust")}</th>
                  <th>{t("assets.updated")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((a) => (
                  <tr key={a.id}>
                    <td>{a.project_name ?? "—"}</td>
                    <td className="text-secondary">
                      <AssetLink
                        asset={a}
                        onPreviewReport={
                          a.asset_kind === "report" ? () => setReportPreviewId(a.backend_id) : undefined
                        }
                      />
                    </td>
                    <td>{t(`assets.kinds.${a.asset_kind}`)}</td>
                    <td className="text-xs text-secondary">
                      {t(`assets.sources.${a.source_type}`)}
                    </td>
                    <td>
                      <StatusBadge status={a.reuse_state} />
                    </td>
                    <td>
                      {a.session_id ? (
                        <Link
                          to="/conversations"
                          search={sessionChatSearch(a.session_id, projectId || undefined)}
                          className="inline-flex items-center gap-1 no-underline hover:underline"
                        >
                          {t("assets.view")}
                          {a.session_trusted_status === "blocked" && (
                            <StatusBadge status="blocked" />
                          )}
                        </Link>
                      ) : (
                        "—"
                      )}
                    </td>
                    <td>
                      <StatusBadge status={a.trust_level} />
                    </td>
                    <td className="text-secondary text-xs">{a.updated_at ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {reportPreviewId && (
        <SectionCard
          title={reportPreview.data?.artifact?.artifact?.title ?? t("assets.reportPreview")}
          className="mt-4"
          action={
            <button
              type="button"
              className="dw-btn-secondary text-xs"
              onClick={() => setReportPreviewId("")}
            >
              {t("reports.close")}
            </button>
          }
        >
          {reportPreview.isLoading && (
            <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
          )}
          {!reportPreview.isLoading && reportPreview.data?.artifact?.report_markdown && (
            <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-[480px] whitespace-pre-wrap m-0">
              {reportPreview.data.artifact.report_markdown}
            </pre>
          )}
          {!reportPreview.isLoading && !reportPreview.data?.artifact?.report_markdown && (
            <p className="text-sm text-secondary m-0">{t("assets.reportPreviewEmpty")}</p>
          )}
        </SectionCard>
      )}
    </CcPageShell>
  );
}

function AssetLink({
  asset,
  onPreviewReport,
}: {
  asset: AssetItem;
  onPreviewReport?: () => void;
}) {
  const t = useT();
  if (asset.asset_kind === "report") {
    if (onPreviewReport) {
      return (
        <button
          type="button"
          className="font-code text-xs text-left border-0 bg-transparent p-0 cursor-pointer text-primary hover:underline"
          onClick={onPreviewReport}
        >
          {asset.title}
        </button>
      );
    }
    return (
      <ControlCenterLink
        to="/reports"
        search={{ artifact_id: asset.backend_id }}
        className="font-code text-xs no-underline hover:underline"
      >
        {asset.title}
      </ControlCenterLink>
    );
  }
  if (asset.backend_type === "skill") {
    return (
      <ControlCenterLink
        to="/assets/$artifactId"
        params={{ artifactId: asset.id }}
        className="font-code text-xs no-underline hover:underline"
      >
        {asset.title}
      </ControlCenterLink>
    );
  }
  return (
    <ControlCenterLink
      to="/assets/$artifactId"
      params={{ artifactId: asset.id }}
      className="font-code text-xs no-underline hover:underline"
      title={asset.path ?? asset.subtitle}
    >
      {asset.title || asset.path || t("assets.unnamed")}
    </ControlCenterLink>
  );
}

function exportAssets(rows: AssetItem[], t: (k: string) => string) {
  const header = [
    t("assets.project"),
    t("assets.name"),
    t("assets.type"),
    t("assets.source"),
    t("assets.reuseState"),
    t("conversations.trust"),
    t("assets.updated"),
  ];
  const data = rows.map((a) => [
    a.project_name ?? "",
    a.title,
    a.asset_kind,
    a.source_type,
    a.reuse_state,
    a.trust_level,
    a.updated_at ?? "",
  ]);
  downloadCsv(`assets-${new Date().toISOString().slice(0, 10)}.csv`, [header, ...data]);
}
