import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { downloadCsv } from "@/utils/exportCsv";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type TrustFilter = "all" | "unverified" | "blocked";

export function AssetsPage() {
  const t = useT();
  const { trust: trustSearch } = useSearch({ from: "/_shell/assets" });
  const projects = useQuery({
    queryKey: ["projects"],
    queryFn: api.projects,
  });
  const list = projects.data?.projects ?? [];
  const [projectId, setProjectId] = useState<string>("");
  const [kind, setKind] = useState<string>("");
  const [trustFilter, setTrustFilter] = useState<TrustFilter>(
    trustSearch ?? "all",
  );

  useEffect(() => {
    if (trustSearch) setTrustFilter(trustSearch);
  }, [trustSearch]);

  const artifacts = useQuery({
    queryKey: ["artifacts", projectId, kind, trustFilter],
    queryFn: () =>
      api.artifacts({
        projectId: projectId || undefined,
        kind: kind || undefined,
        unverifiedOnly: trustFilter === "unverified",
        blockedSessionOnly: trustFilter === "blocked",
        limit: 100,
      }),
  });

  const rows = artifacts.data?.artifacts ?? [];

  const trustFilters = [
    { id: "all" as const, label: t("assets.filterAll") },
    { id: "unverified" as const, label: t("assets.filterUnverified") },
    { id: "blocked" as const, label: t("assets.filterBlocked") },
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
          rows.length > 0 ? (
            <button
              type="button"
              className="dw-btn-secondary"
              onClick={() => exportArtifacts(rows, t)}
            >
              <Icon name="download" size={16} />
              {t("assets.export")}
            </button>
          ) : undefined
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
        <select className="dw-input" value={kind} onChange={(e) => setKind(e.target.value)}>
          <option value="">{t("assets.allTypes")}</option>
          <option value="file">{t("assets.kinds.file")}</option>
          <option value="notebook">{t("assets.kinds.notebook")}</option>
          <option value="report">{t("assets.kinds.report")}</option>
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

      {rows.length === 0 && !artifacts.isLoading && (
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
                  <th>{t("common.path")}</th>
                  <th>{t("assets.type")}</th>
                  <th>{t("audit.session")}</th>
                  <th>{t("conversations.trust")}</th>
                  <th>{t("assets.gate")}</th>
                  <th>{t("assets.updated")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((a) => (
                  <tr key={a.id}>
                    <td>{a.project_name ?? "—"}</td>
                    <td className="text-secondary">
                      <Link
                        to="/assets/$artifactId"
                        params={{ artifactId: a.id }}
                        className="font-code text-xs no-underline hover:underline"
                      >
                        {a.path}
                      </Link>
                    </td>
                    <td>{a.kind}</td>
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
                    <td className="text-secondary text-xs">{a.verified_by_gate_name ?? "—"}</td>
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

function exportArtifacts(
  rows: import("@/api/types").ArtifactRecord[],
  t: (k: string) => string,
) {
  const header = [
    t("assets.project"),
    t("common.path"),
    t("assets.type"),
    t("conversations.trust"),
    t("assets.gate"),
    t("assets.updated"),
  ];
  const data = rows.map((a) => [
    a.project_name ?? "",
    a.path,
    a.kind,
    a.trust_level,
    a.verified_by_gate_name ?? "",
    a.updated_at ?? "",
  ]);
  downloadCsv(`assets-${new Date().toISOString().slice(0, 10)}.csv`, [header, ...data]);
}
