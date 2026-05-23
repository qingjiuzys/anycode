import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type { RuntimeSettings } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SkillsGovernancePanel({ runtime }: { runtime?: RuntimeSettings }) {
  const t = useT();
  const qc = useQueryClient();
  const skills = useQuery({ queryKey: ["skills"], queryFn: () => api.skills(100) });

  const rescan = useMutation({
    mutationFn: api.rescanSkills,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["skills"] });
      qc.invalidateQueries({ queryKey: ["overview"] });
      qc.invalidateQueries({ queryKey: ["runtime-settings"] });
    },
  });

  const setAll = useMutation({
    mutationFn: ({ skillId, enabled }: { skillId: string; enabled: boolean }) =>
      api.setSkillAllProjects(skillId, enabled),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["skills"] });
      qc.invalidateQueries({ queryKey: ["project-skills"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const list = skills.data?.skills ?? [];

  return (
    <>
      <SectionCard title={t("settings.tabs.skills")}>
        <div className="grid grid-cols-2 gap-4 mb-4">
          <div className="dw-stat-card">
            <div className="dw-stat-value">{runtime?.skills_total ?? "…"}</div>
            <div className="dw-stat-label">{t("settings.skillsGov.total")}</div>
          </div>
          <div className="dw-stat-card">
            <div className="dw-stat-value">{runtime?.skills_enabled_links ?? "…"}</div>
            <div className="dw-stat-label">{t("settings.skillsGov.enabledLinks")}</div>
          </div>
        </div>
        <div className="flex flex-wrap gap-2 mb-2">
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={rescan.isPending}
            onClick={() => rescan.mutate()}
          >
            {rescan.isPending ? t("agents.rescanning") : t("agents.rescan")}
          </button>
          <Link to="/agents" className="dw-btn-secondary no-underline">
            {t("settings.skillsGov.rescanLink")} →
          </Link>
        </div>
        {rescan.isSuccess && (
          <p className="text-sm text-secondary m-0">
            {t("agents.rescanSuccess").replace("{n}", String(rescan.data.skills_synced))}
          </p>
        )}
        <p className="text-sm text-secondary m-0 mt-2">{t("settings.skillsGov.hint")}</p>
      </SectionCard>

      <SectionCard title={t("settings.skillsGov.globalTitle")} noPadding>
        {list.length === 0 ? (
          <p className="text-sm text-secondary px-4 py-4 m-0">{t("agents.emptySkills")}</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="dw-table">
              <thead>
                <tr>
                  <th>{t("common.name")}</th>
                  <th className="text-right">{t("settings.skillsGov.enabledProjects")}</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {list.map((sk) => (
                  <tr key={sk.id}>
                    <td>
                      <Link
                        to="/agents/$skillId"
                        params={{ skillId: sk.id }}
                        className="font-medium no-underline hover:underline"
                      >
                        {sk.name}
                      </Link>
                      {sk.description && (
                        <div className="text-xs text-secondary mt-0.5 line-clamp-1">
                          {sk.description}
                        </div>
                      )}
                    </td>
                    <td className="text-right tabular-nums">{sk.projects_count}</td>
                    <td className="text-right whitespace-nowrap">
                      <button
                        type="button"
                        className="dw-btn-ghost text-xs"
                        disabled={setAll.isPending}
                        onClick={() => setAll.mutate({ skillId: sk.id, enabled: true })}
                      >
                        {t("settings.skillsGov.enableAll")}
                      </button>
                      <button
                        type="button"
                        className="dw-btn-ghost text-xs"
                        disabled={setAll.isPending}
                        onClick={() => setAll.mutate({ skillId: sk.id, enabled: false })}
                      >
                        {t("settings.skillsGov.disableAll")}
                      </button>
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
