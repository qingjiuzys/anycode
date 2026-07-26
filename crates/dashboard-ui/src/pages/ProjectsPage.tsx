import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { InlineRename } from "@/components/InlineRename";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { WorkspacePathsPanel } from "@/components/WorkspacePathsPanel";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { ListPageToolbar } from "@/components/ui/ListPageToolbar";
import { ListPaginationBar } from "@/components/ui/ListPaginationBar";
import { PageHeader } from "@/components/ui/PageHeader";
import { StatusBadge, TrustBar } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";

type StatusFilter = "all" | "active" | "archived" | "error";
const PAGE_SIZE_OPTIONS = [25, 50, 100] as const;

/** Last 1-2 path segments, used to disambiguate same-named projects. */
function rootPathSuffix(rootPath: string): string {
  const segments = rootPath.split(/[\\/]+/).filter(Boolean);
  return segments.slice(-2).join("/");
}

function projectsErrorMessage(error: unknown, t: (key: string) => string): string {
  const message = error instanceof Error ? error.message : String(error ?? "");
  if (/\b401\b/.test(message)) {
    return t("projects.authError");
  }
  return message || t("projects.loadError");
}

export function ProjectsPage(_props: EmbeddedPageProps = {}) {
  const t = useT();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState<number>(PAGE_SIZE_OPTIONS[0]);
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [scanMessage, setScanMessage] = useState<string | null>(null);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(search.trim()), 300);
    return () => clearTimeout(timer);
  }, [search]);

  useEffect(() => {
    setPage(0);
  }, [debouncedSearch, statusFilter]);

  const statusParam =
    statusFilter === "all"
      ? undefined
      : statusFilter === "active"
        ? "active"
        : statusFilter;

  const { data, isLoading, error } = useQuery({
    queryKey: ["projects", debouncedSearch, statusFilter, page, pageSize],
    queryFn: () =>
      api.projects({
        limit: pageSize,
        offset: page * pageSize,
        q: debouncedSearch || undefined,
        status: statusParam,
        sort: "updated_at_desc",
      }),
  });

  const bootstrap = useQuery({
    queryKey: ["bootstrap"],
    queryFn: async () => (await api.bootstrap()).bootstrap,
  });

  const scan = useMutation({
    mutationFn: api.scanProjects,
    onSuccess: (result) => {
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
      void queryClient.invalidateQueries({ queryKey: ["overview"] });
      void queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
      setScanMessage(
        t("projects.scanSuccess")
          .replace("{registered}", String(result.projects_registered))
          .replace("{skills}", String(result.skills_synced)),
      );
    },
  });

  const archive = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      api.patchProjectStatus(id, status),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
  });

  const rename = useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      api.renameProject(id, name),
    onSuccess: (_data, { id }) => {
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
      void queryClient.invalidateQueries({ queryKey: ["project", id] });
    },
  });

  if (error) {
    return <div className="dw-alert-error">{projectsErrorMessage(error, t)}</div>;
  }

  const projects = data?.projects ?? [];
  const total = data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));

  const nameCounts = new Map<string, number>();
  for (const p of projects) {
    nameCounts.set(p.name, (nameCounts.get(p.name) ?? 0) + 1);
  }

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
              className="dw-input dw-input--pill w-full pl-9"
              placeholder={t("projects.searchPlaceholder")}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <select
            className="dw-input dw-input--pill h-[34px] min-w-[120px] shrink-0 pr-8"
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
          >
            <option value="all">{t("projects.statusAll")}</option>
            <option value="active">{t("projects.statusActive")}</option>
            <option value="archived">{t("projects.statusArchived")}</option>
            <option value="error">{t("projects.statusError")}</option>
          </select>
        </>
      }
      actions={
        <>
          <button
            type="button"
            className="dw-btn-secondary dw-btn--pill"
            onClick={() => setNewProjectOpen(true)}
          >
            <Icon name="add" size={16} />
            {t("projects.newProject")}
          </button>
          <button
            type="button"
            className="dw-btn-primary dw-btn--pill"
            disabled={scan.isPending}
            onClick={() => scan.mutate()}
          >
            <Icon name="radar" size={16} />
            {scan.isPending ? t("common.loading") : t("projects.scanNew")}
          </button>
        </>
      }
    />
  );

  return (
    <>
      <NewProjectDialog open={newProjectOpen} onClose={() => setNewProjectOpen(false)} />

      <CcPageShell
        header={
          <>
            <PageHeader
              title={t("projects.title")}
              subtitle={t("projects.subtitle")}
              breadcrumbs={[{ label: t("nav.home"), to: "/" }, { label: t("projects.title") }]}
            />
            {scanMessage && (
              <p className="text-sm text-secondary m-0 mt-3 bg-surface-container-low border border-outline-variant rounded-lg px-4 py-2">
                {scanMessage}
              </p>
            )}
          </>
        }
      >
        <div className="mb-4">
          <WorkspacePathsPanel bootstrap={bootstrap.data} />
        </div>

        {isLoading && <p className="text-secondary text-sm">{t("common.loading")}</p>}

        {!isLoading && projects.length === 0 && (
          <>
            {filterToolbar}
            <EmptyState
              title={t("projects.emptyTitle")}
              description={t("projects.emptyDesc")}
              icon="folder_off"
              actions={
                <button
                  type="button"
                  className="dw-btn-primary"
                  disabled={scan.isPending}
                  onClick={() => scan.mutate()}
                >
                  <Icon name="radar" size={16} />
                  {scan.isPending ? t("common.loading") : t("projects.scanNew")}
                </button>
              }
            />
          </>
        )}

        {projects.length > 0 && (
          <div className="dw-section-card dw-list-card">
            <div className="dw-list-card__toolbar px-3 py-3 border-b border-outline-variant/40">
              {filterToolbar}
            </div>
            <div className="dw-list-card__scroll">
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
                    <th className="text-right">{t("common.actions")}</th>
                  </tr>
                </thead>
                <tbody>
                  {projects.map((p) => (
                    <tr key={p.id} className="group">
                      <td>
                        <InlineRename
                          value={p.name}
                          label={t("projects.rename")}
                          disabled={rename.isPending}
                          onSave={(name) => rename.mutate({ id: p.id, name })}
                        >
                          <ControlCenterLink
                            to="/projects/$projectId"
                            params={{ projectId: p.id }}
                            className="flex items-center gap-2 font-medium no-underline hover:underline"
                          >
                            <div className="w-8 h-8 rounded-full bg-primary-fixed flex items-center justify-center text-primary shrink-0">
                              <Icon name="folder" size={16} />
                            </div>
                            {p.name}
                            {(nameCounts.get(p.name) ?? 0) > 1 && (
                              <span className="text-[11px] text-outline font-code font-normal">
                                · {rootPathSuffix(p.root_path)}
                              </span>
                            )}
                          </ControlCenterLink>
                        </InlineRename>
                      </td>
                      <td>
                        <div className="flex flex-col gap-1 max-w-[240px]">
                          <span className="font-code text-secondary truncate block">
                            {p.root_path}
                          </span>
                          {p.root_exists === false && (
                            <span className="text-[10px] text-warn">{t("projects.rootMissing")}</span>
                          )}
                        </div>
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
                      <td className="text-right">
                        {p.status !== "archived" ? (
                          <button
                            type="button"
                            className="dw-btn-secondary text-xs"
                            disabled={archive.isPending}
                            onClick={() => archive.mutate({ id: p.id, status: "archived" })}
                          >
                            {t("projects.archive")}
                          </button>
                        ) : (
                          <button
                            type="button"
                            className="dw-btn-secondary text-xs"
                            disabled={archive.isPending}
                            onClick={() => archive.mutate({ id: p.id, status: "active" })}
                          >
                            {t("projects.restore")}
                          </button>
                        )}
                      </td>
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
                pageSizeLabel={t("projects.pageSizeLabel")}
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
    </>
  );
}
