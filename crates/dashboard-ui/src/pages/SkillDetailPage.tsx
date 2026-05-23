import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { api } from "@/api/client";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function SkillDetailPage() {
  const t = useT();
  const qc = useQueryClient();
  const { skillId } = useParams({ from: "/_shell/agents/$skillId" });
  const skill = useQuery({
    queryKey: ["skill", skillId],
    queryFn: () => api.skillDetail(skillId),
  });

  const setAll = useMutation({
    mutationFn: (enabled: boolean) => api.setSkillAllProjects(skillId, enabled),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["skill", skillId] });
      qc.invalidateQueries({ queryKey: ["skills"] });
      qc.invalidateQueries({ queryKey: ["project-skills"] });
    },
  });

  const toggleProject = useMutation({
    mutationFn: ({ projectId, enabled }: { projectId: string; enabled: boolean }) =>
      api.setProjectSkill(projectId, skillId, enabled),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["skill", skillId] }),
  });

  const s = skill.data?.skill;
  if (skill.isLoading) return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  if (!s) return <div className="dw-alert-error">{t("skillDetail.notFound")}</div>;

  const perms = s.permissions as {
    read_dirs?: string[];
    write_dirs?: string[];
    network?: boolean;
  };

  return (
    <>
      <PageHeader
        title={s.name}
        subtitle={s.source_path}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("agents.title"), to: "/agents" },
          { label: s.name },
        ]}
        meta={
          <>
            <span>
              {t("skillDetail.enabledCount").replace("{n}", String(s.projects_count))}
            </span>
            <span className="text-outline-variant">·</span>
            <button
              type="button"
              className="dw-btn-ghost text-xs"
              disabled={setAll.isPending}
              onClick={() => setAll.mutate(true)}
            >
              {t("settings.skillsGov.enableAll")}
            </button>
            <button
              type="button"
              className="dw-btn-ghost text-xs"
              disabled={setAll.isPending}
              onClick={() => setAll.mutate(false)}
            >
              {t("settings.skillsGov.disableAll")}
            </button>
          </>
        }
      />
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SectionCard title={t("skillDetail.descPerms")}>
          <p className="text-sm m-0 mb-4">{s.description || "—"}</p>
          <dl className="grid grid-cols-[minmax(4rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
            <dt className="text-secondary font-medium m-0">{t("skillDetail.readOnly")}</dt>
            <dd className="m-0">{(perms.read_dirs ?? []).join(", ") || "—"}</dd>
            <dt className="text-secondary font-medium m-0">{t("skillDetail.writable")}</dt>
            <dd className="m-0">{(perms.write_dirs ?? []).join(", ") || "—"}</dd>
            <dt className="text-secondary font-medium m-0">{t("skillDetail.network")}</dt>
            <dd className="m-0">{perms.network ? t("projectDetail.yes") : t("projectDetail.no")}</dd>
          </dl>
        </SectionCard>
        <SectionCard title={t("skillDetail.recentRuns")}>
          {s.recent_runs.length === 0 && (
            <p className="text-sm text-secondary m-0">{t("skillDetail.noRuns")}</p>
          )}
          <ul className="m-0 pl-5 text-sm space-y-2">
            {s.recent_runs.map((r) => (
              <li key={r.id} className="flex flex-wrap items-center gap-2">
                <StatusBadge status={r.status} />
                <span className="text-secondary">{r.started_at}</span>
                {r.session_id && (
                  <Link
                    to="/sessions/$sessionId"
                    params={{ sessionId: r.session_id }}
                    className="text-xs no-underline hover:underline"
                  >
                    {t("audit.session")}
                  </Link>
                )}
              </li>
            ))}
          </ul>
        </SectionCard>
      </div>

      <SectionCard title={t("skillDetail.projectLinks")} noPadding>
        {s.projects.length === 0 ? (
          <p className="text-sm text-secondary px-4 py-4 m-0">{t("skillDetail.noProjects")}</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("audit.project")}</th>
                  <th>{t("common.status")}</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {s.projects.map((p) => (
                  <tr key={p.project_id}>
                    <td>
                      <Link
                        to="/projects/$projectId"
                        params={{ projectId: p.project_id }}
                        className="font-medium no-underline hover:underline"
                      >
                        {p.project_name}
                      </Link>
                    </td>
                    <td>
                      <StatusBadge status={p.enabled ? "ok" : "cancelled"} />
                    </td>
                    <td className="text-right">
                      <input
                        type="checkbox"
                        checked={p.enabled}
                        disabled={toggleProject.isPending}
                        onChange={(e) =>
                          toggleProject.mutate({
                            projectId: p.project_id,
                            enabled: e.target.checked,
                          })
                        }
                        className="accent-primary"
                        aria-label={p.project_name}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SectionCard>
    </>
  );
}
