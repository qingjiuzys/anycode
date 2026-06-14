import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { api } from "@/api/client";
import type { AssetItem } from "@/api/types/artifacts";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { downloadCsv } from "@/utils/exportCsv";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

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

export function AssetsPage() {
  const t = useT();
  const queryClient = useQueryClient();
  const { trust: trustSearch } = useSearch({ from: "/_shell/assets" });
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

  useEffect(() => {
    if (trustSearch) setTrustFilter(trustSearch);
  }, [trustSearch]);

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
    { value: "agent_created", label: t("assets.sources.agentCreated") },
    { value: "workspace_scan", label: t("assets.sources.workspaceScan") },
    { value: "report_archive", label: t("assets.sources.reportArchive") },
    { value: "skill_scan", label: t("assets.sources.skillScan") },
    { value: "workflow_scan", label: t("assets.sources.workflowScan") },
  ];

  const reuseOptions: { value: ReuseFilter; label: string }[] = [
    { value: "", label: t("assets.allReuseStates") },
    { value: "candidate", label: t("assets.reuseStates.candidate") },
    { value: "reusable", label: t("assets.reuseStates.reusable") },
    { value: "archived", label: t("assets.reuseStates.archived") },
  ];

  return (
    <>
      <PageHeader
        title={t("assets.title")}
        subtitle={t("assets.subtitle")}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("assets.title") },
        ]}
        actions={
          <>
            {projectId && (
              <button
                type="button"
                className="dw-btn-secondary"
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
                className="dw-btn-secondary"
                onClick={() => exportAssets(rows, t)}
              >
                <Icon name="download" size={16} />
                {t("assets.export")}
              </button>
            )}
          </>
        }
      />

      <div className="flex flex-wrap items-center gap-2 mb-3">
        <select
          className="dw-input"
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
          className="dw-input"
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
          className="dw-input"
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
          className="dw-input"
          value={reuseState}
          onChange={(e) => setReuseState(e.target.value as ReuseFilter)}
        >
          {reuseOptions.map((o) => (
            <option key={o.value || "all-reuse"} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </div>

      <div className="flex flex-wrap gap-2 mb-4">
        {trustFilters.map((f) => (
          <button
            key={f.id}
            type="button"
            className={`dw-chip${trustFilter === f.id ? " active" : ""}`}
            onClick={() => setTrustFilter(f.id)}
          >
            {f.label}
          </button>
        ))}
      </div>

      {rows.length === 0 && !assets.isLoading && (
        <EmptyState
          title={t("assets.emptyTitle")}
          description={t("assets.emptyDesc")}
          icon="inventory_2"
        />
      )}

      {rows.length > 0 && (
        <div className="dw-section-card overflow-hidden">
          <div className="overflow-x-auto">
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
                      <AssetLink asset={a} />
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
                          to="/sessions/$sessionId"
                          params={{ sessionId: a.session_id }}
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
    </>
  );
}

function AssetLink({ asset }: { asset: AssetItem }) {
  const t = useT();
  if (asset.asset_kind === "report") {
    return (
      <Link
        to="/reports"
        search={{ artifact_id: asset.backend_id }}
        className="font-code text-xs no-underline hover:underline"
      >
        {asset.title}
      </Link>
    );
  }
  if (asset.backend_type === "skill") {
    return (
      <Link
        to="/assets/$artifactId"
        params={{ artifactId: asset.id }}
        className="font-code text-xs no-underline hover:underline"
      >
        {asset.title}
      </Link>
    );
  }
  return (
    <Link
      to="/assets/$artifactId"
      params={{ artifactId: asset.id }}
      className="font-code text-xs no-underline hover:underline"
      title={asset.path ?? asset.subtitle}
    >
      {asset.title || asset.path || t("assets.unnamed")}
    </Link>
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
