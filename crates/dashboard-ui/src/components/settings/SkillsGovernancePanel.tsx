import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import type { RuntimeSettings } from "@/api/types";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SkillsImportDialog } from "@/components/SkillsImportDialog";
import { ListPaginationBar } from "@/components/ui/ListPaginationBar";
import { SectionCard } from "@/components/ui/SectionCard";
import { useLocale, useT } from "@/i18n/context";
import {
  SKILL_CATEGORIES,
  filterSkillsByCategory,
  normalizeSkillCategory,
  skillDisplayDescription,
  skillDisplayName,
  type SkillCategory,
} from "@/lib/skillCatalog";

const PAGE_SIZES = [10, 20, 50];

export function SkillsGovernancePanel({ runtime }: { runtime?: RuntimeSettings }) {
  const t = useT();
  const locale = useLocale();
  const qc = useQueryClient();
  const [importOpen, setImportOpen] = useState(false);
  const [categoryFilter, setCategoryFilter] = useState<SkillCategory | "all">("all");
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(PAGE_SIZES[0]!);
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
  const filtered = useMemo(
    () => filterSkillsByCategory(list, categoryFilter),
    [list, categoryFilter],
  );
  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const pageItems = useMemo(
    () => filtered.slice(page * pageSize, page * pageSize + pageSize),
    [filtered, page, pageSize],
  );

  useEffect(() => {
    setPage(0);
  }, [categoryFilter, pageSize]);

  useEffect(() => {
    if (page > 0 && page >= pageCount) setPage(Math.max(0, pageCount - 1));
  }, [page, pageCount]);

  return (
    <>
      <SectionCard
        title={t("settings.tabs.skills")}
        action={
          <button
            type="button"
            className="dw-btn-secondary text-[13px]"
            onClick={() => setImportOpen(true)}
          >
            <Icon name="upload" size={14} className="inline mr-1" />
            {t("settings.skillsGov.importBtn")}
          </button>
        }
      >
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
        {list.length > 0 && (
          <div
            className="flex flex-wrap gap-1.5 px-4 pt-4 pb-2 sticky top-0 z-[1] bg-surface-container-lowest border-b border-outline-variant/30"
            role="tablist"
          >
            <FilterPill
              active={categoryFilter === "all"}
              label={t("agents.skillCategory.all")}
              onClick={() => setCategoryFilter("all")}
            />
            {SKILL_CATEGORIES.map((cat) => (
              <FilterPill
                key={cat}
                active={categoryFilter === cat}
                label={t(`agents.skillCategory.${cat}`)}
                onClick={() => setCategoryFilter(cat)}
              />
            ))}
          </div>
        )}
        {list.length === 0 ? (
          <p className="text-sm text-secondary px-4 py-4 m-0">{t("agents.emptySkills")}</p>
        ) : filtered.length === 0 ? (
          <p className="text-sm text-secondary px-4 py-4 m-0">{t("agents.skillMarketEmpty")}</p>
        ) : (
          <>
            <div className="overflow-x-auto">
              <table className="dw-table">
                <thead>
                  <tr>
                    <th>{t("common.name")}</th>
                    <th>{t("settings.skillsGov.categoryCol")}</th>
                    <th className="text-right">{t("settings.skillsGov.enabledProjects")}</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {pageItems.map((sk) => (
                    <tr key={sk.id}>
                      <td>
                        <Link
                          to="/agents/$skillId"
                          params={{ skillId: sk.id }}
                          className="font-medium no-underline hover:underline text-on-surface"
                        >
                          {skillDisplayName(sk, locale)}
                        </Link>
                        {skillDisplayDescription(sk, locale) && (
                          <div className="text-[13px] text-secondary mt-0.5 line-clamp-1">
                            {skillDisplayDescription(sk, locale)}
                          </div>
                        )}
                      </td>
                      <td>
                        <span className="text-[13px] text-secondary">
                          {t(`agents.skillCategory.${normalizeSkillCategory(sk.category)}`)}
                        </span>
                      </td>
                      <td className="text-right tabular-nums">{sk.projects_count}</td>
                      <td className="text-right whitespace-nowrap">
                        <button
                          type="button"
                          className="dw-btn-ghost text-[13px]"
                          disabled={setAll.isPending}
                          onClick={() => setAll.mutate({ skillId: sk.id, enabled: true })}
                        >
                          {t("settings.skillsGov.enableAll")}
                        </button>
                        <button
                          type="button"
                          className="dw-btn-ghost text-[13px]"
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
            <ListPaginationBar
              page={page}
              pageCount={pageCount}
              pageSize={pageSize}
              pageSizeOptions={PAGE_SIZES}
              total={filtered.length}
              onPageChange={setPage}
              onPageSizeChange={setPageSize}
            />
          </>
        )}
      </SectionCard>

      <SkillsImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
    </>
  );
}

function FilterPill({
  active,
  label,
  onClick,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`dw-chip text-[13px]${active ? " active" : ""}`}
      onClick={onClick}
    >
      {label}
    </button>
  );
}
