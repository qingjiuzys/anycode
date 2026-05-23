import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge, TrustBar } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

type StatusFilter = "all" | "active" | "archived" | "error";

export function ProjectsPage() {
  const t = useT();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  const { data, isLoading, error } = useQuery({
    queryKey: ["projects"],
    queryFn: api.projects,
  });

  const scan = useMutation({
    mutationFn: api.scanProjects,
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
      void queryClient.invalidateQueries({ queryKey: ["overview"] });
      setScanMessage(
        t("projects.scanSuccess")
          .replace("{registered}", String(result.projects_registered))
          .replace("{ingested}", String(result.ingested_tasks))
          .replace("{skills}", String(result.skills_synced)),
      );
    },
  });

  if (error) {
    return <div className="dw-alert-error">{(error as Error).message}</div>;
  }

  const all = data?.projects ?? [];
  const filtered = all.filter((p) => {
    const q = search.trim().toLowerCase();
    const matchesSearch =
      !q ||
      p.name.toLowerCase().includes(q) ||
      p.root_path.toLowerCase().includes(q);
    const st = p.status.toLowerCase();
    const matchesStatus =
      statusFilter === "all" ||
      (statusFilter === "active" && (st === "active" || st === "running" || st === "ok")) ||
      (statusFilter === "archived" && st === "archived") ||
      (statusFilter === "error" && (st === "error" || st === "failed" || st === "blocked"));
    return matchesSearch && matchesStatus;
  });

  const showingLabel = t("projects.showing")
    .replace("{shown}", String(filtered.length))
    .replace("{total}", String(all.length));

  return (
    <>
      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />

      <PageHeader
        title={t("projects.title")}
        subtitle={t("projects.subtitle")}
        breadcrumbs={[{ label: t("breadcrumb.home"), to: "/" }, { label: t("projects.title") }]}
        actions={
          <>
            <button
              type="button"
              className={`dw-btn-secondary${statusFilter !== "all" ? " ring-1 ring-primary" : ""}`}
              onClick={() =>
                setStatusFilter((f) => (f === "all" ? "active" : f === "active" ? "all" : "active"))
              }
              title={t("projects.statusFilterHint")}
            >
              <Icon name="filter_list" size={16} />
              {t("common.filter")}
            </button>
            <button
              type="button"
              className="dw-btn-secondary"
              onClick={() => setNewProjectOpen(true)}
            >
              <Icon name="add" size={16} />
              {t("projects.newProject")}
            </button>
            <button
              type="button"
              className="dw-btn-primary"
              disabled={scan.isPending}
              onClick={() => scan.mutate()}
            >
              <Icon name="radar" size={16} />
              {scan.isPending ? t("common.loading") : t("projects.scanNew")}
            </button>
          </>
        }
      />

      {scanMessage && (
        <p className="text-sm text-secondary m-0 bg-surface-container-low border border-outline-variant rounded-lg px-4 py-2">
          {scanMessage}
        </p>
      )}

      <div className="flex flex-col sm:flex-row items-center justify-between gap-4 bg-surface-container-lowest border border-outline-variant rounded-lg p-2 shadow-sm">
        <div className="flex items-center gap-2 w-full sm:w-auto flex-1">
          <div className="relative flex-1 sm:max-w-xs">
            <Icon
              name="search"
              size={16}
              className="absolute left-2 top-1/2 -translate-y-1/2 text-outline"
            />
            <input
              className="dw-input w-full pl-8"
              placeholder={t("projects.searchPlaceholder")}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <select
            className="dw-input h-[34px] min-w-[120px]"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
          >
            <option value="all">{t("projects.statusAll")}</option>
            <option value="active">{t("projects.statusActive")}</option>
            <option value="archived">{t("projects.statusArchived")}</option>
            <option value="error">{t("projects.statusError")}</option>
          </select>
        </div>
        <div className="text-xs text-secondary px-2 shrink-0">{showingLabel}</div>
      </div>

      {isLoading && <p className="text-secondary text-sm">{t("common.loading")}</p>}

      {!isLoading && filtered.length === 0 && (
        <EmptyState
          title={t("projects.emptyTitle")}
          description={t("projects.emptyDesc")}
          icon="folder_off"
        />
      )}

      {filtered.length > 0 && (
        <div className="dw-section-card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("common.name")}</th>
                  <th>{t("projects.rootPath")}</th>
                  <th>{t("common.status")}</th>
                  <th>{t("projects.trust")}</th>
                  <th className="text-right">{t("projects.sessions")}</th>
                  <th className="text-right">{t("nav.assets")}</th>
                  <th className="text-right">{t("home.lastActivity")}</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((p) => (
                  <tr key={p.id} className="group">
                    <td>
                      <Link
                        to="/projects/$projectId"
                        params={{ projectId: p.id }}
                        className="flex items-center gap-2 font-medium no-underline hover:underline"
                      >
                        <div className="w-8 h-8 rounded-full bg-primary-fixed flex items-center justify-center text-primary shrink-0">
                          <Icon name="folder" size={16} />
                        </div>
                        {p.name}
                      </Link>
                    </td>
                    <td>
                      <span className="font-code text-secondary truncate block max-w-[220px]">
                        {p.root_path}
                      </span>
                    </td>
                    <td>
                      <StatusBadge status={p.status} />
                    </td>
                    <td>
                      <TrustBar score={p.trust_score} />
                    </td>
                    <td className="text-right">{p.sessions_count}</td>
                    <td className="text-right">{p.artifacts_count}</td>
                    <td className="text-right text-secondary text-xs">{p.updated_at}</td>
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
