import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { useMemo, useState } from "react";
import type { SkillRecord } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { useLocale, useT } from "@/i18n/context";
import {
  categoriesWithEntries,
  filterSkillsByCategory,
  groupSkillsByCategory,
  normalizeSkillCategory,
  skillDisplayDescription,
  skillMatchesSearch,
  type SkillCategory,
} from "@/lib/skillCatalog";

type Props = {
  skills: SkillRecord[];
  loading?: boolean;
  rescanPending?: boolean;
  onRescan?: () => void;
  rescanSuccess?: number;
  missingStarterCount?: number;
  onInstallStarter?: () => void;
  installStarterPending?: boolean;
};

export function InstalledSkillsPanel({
  skills,
  loading,
  rescanPending,
  onRescan,
  rescanSuccess,
  missingStarterCount = 0,
  onInstallStarter,
  installStarterPending,
}: Props) {
  const t = useT();
  const locale = useLocale();
  const [categoryFilter, setCategoryFilter] = useState<SkillCategory | "all">("all");
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState<Set<SkillCategory>>(new Set());

  const filtered = useMemo(() => {
    let list = skills.filter((s) => skillMatchesSearch(s, search));
    list = filterSkillsByCategory(list, categoryFilter);
    return list;
  }, [skills, search, categoryFilter]);

  const visibleCategories = useMemo(() => categoriesWithEntries(skills), [skills]);
  const groups = useMemo(() => groupSkillsByCategory(filtered), [filtered]);
  const allExpanded = groups.length > 0 && groups.every((g) => expanded.has(g.category));

  function toggleCategory(cat: SkillCategory) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(cat)) next.delete(cat);
      else next.add(cat);
      return next;
    });
  }

  function toggleAll() {
    if (allExpanded) {
      setExpanded(new Set());
    } else {
      setExpanded(new Set(groups.map((g) => g.category)));
    }
  }

  return (
    <section className="dw-agents-panel dw-agents-panel--skills" aria-labelledby="agents-skills-heading">
      <header className="dw-agents-panel__head">
        <div>
          <h2 id="agents-skills-heading" className="dw-agents-panel__title">
            {t("agents.skills")}
          </h2>
          {!loading && (
            <p className="dw-agents-panel__sub m-0">
              {skills.length > 0
                ? t("agents.skillsSyncedCount").replace("{n}", String(skills.length))
                : t("agents.skillsSyncedNone")}
            </p>
          )}
        </div>
        {onRescan && (
          <button
            type="button"
            className="dw-btn-secondary text-sm shrink-0"
            disabled={rescanPending}
            onClick={onRescan}
          >
            <Icon name="refresh" size={16} />
            {rescanPending ? t("agents.rescanning") : t("agents.rescan")}
          </button>
        )}
      </header>

      {skills.length > 0 && (
        <div className="px-4 pt-3 pb-2 space-y-2 border-b border-outline-variant/40">
          <div className="relative">
            <Icon
              name="search"
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-outline"
            />
            <input
              type="search"
              className="dw-input w-full pl-9 text-sm"
              placeholder={t("agents.skillMarketSearch")}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>
          <div className="flex flex-wrap items-center gap-1.5">
            <FilterPill
              active={categoryFilter === "all"}
              label={t("agents.skillCategory.all")}
              onClick={() => setCategoryFilter("all")}
            />
            {visibleCategories.map((cat) => (
              <FilterPill
                key={cat}
                active={categoryFilter === cat}
                label={t(`agents.skillCategory.${cat}`)}
                onClick={() => setCategoryFilter(cat)}
              />
            ))}
            {groups.length > 1 && (
              <button type="button" className="dw-btn-ghost text-[10px] ml-auto" onClick={toggleAll}>
                {allExpanded ? t("agents.skillsCollapseAll") : t("agents.skillsExpandAll")}
              </button>
            )}
          </div>
        </div>
      )}

      <div className="dw-agents-panel__body dw-agents-panel__body--list dw-agents-panel__body--scroll">
        {rescanSuccess !== undefined && (
          <p className="dw-agents-toast m-0" role="status">
            <Icon name="check_circle" size={16} className="text-success" />
            {t("agents.rescanSuccess").replace("{n}", String(rescanSuccess))}
          </p>
        )}
        {loading ? (
          <p className="text-sm text-secondary m-0 px-4 py-6">{t("common.loading")}</p>
        ) : skills.length === 0 ? (
          <div className="px-4 py-2">
            <EmptyState
              title={t("agents.emptySkillsTitle")}
              description={t("agents.emptySkills")}
              icon="extension"
              compact
              actions={
                <>
                  {missingStarterCount > 0 && onInstallStarter && (
                    <button
                      type="button"
                      className="dw-btn-primary text-sm"
                      disabled={installStarterPending}
                      onClick={onInstallStarter}
                    >
                      <Icon name="download" size={16} />
                      {installStarterPending ? t("agents.rescanning") : t("agents.installStarterBtn")}
                    </button>
                  )}
                  {onRescan && (
                    <button
                      type="button"
                      className="dw-btn-secondary text-sm"
                      disabled={rescanPending}
                      onClick={onRescan}
                    >
                      <Icon name="refresh" size={16} />
                      {t("agents.rescan")}
                    </button>
                  )}
                </>
              }
            />
          </div>
        ) : filtered.length === 0 ? (
          <p className="text-sm text-secondary m-0 px-4 py-6">{t("agents.skillMarketEmpty")}</p>
        ) : (
          <div>
            {groups.map((group) => {
              const isOpen = expanded.has(group.category);
              return (
                <div key={group.category}>
                  <button
                    type="button"
                    className={`dw-agents-skill-group__head w-full ${isOpen ? "dw-agents-skill-group__head--open" : ""}`}
                    onClick={() => toggleCategory(group.category)}
                    aria-expanded={isOpen}
                  >
                    <Icon
                      name="expand_more"
                      size={18}
                      className="dw-agents-skill-group__chevron shrink-0 text-outline"
                    />
                    <span>{t(`agents.skillCategory.${group.category}`)}</span>
                    <span className="font-normal tabular-nums text-outline ml-1">
                      {group.items.length}
                    </span>
                  </button>
                  {isOpen && (
                    <ul className="dw-agents-skill-list m-0 p-0 list-none">
                      {group.items.map((skill) => (
                        <SkillRow
                          key={skill.id}
                          skill={skill}
                          locale={locale}
                          projectsLabel={t("agents.projectsCount")}
                        />
                      ))}
                    </ul>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </section>
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
      className={`text-[10px] px-2 py-0.5 rounded-full border transition-colors ${
        active
          ? "bg-primary/15 border-primary/40 text-primary font-medium"
          : "border-outline-variant text-secondary hover:bg-surface-container-low"
      }`}
      onClick={onClick}
    >
      {label}
    </button>
  );
}

function SkillRow({
  skill,
  locale,
  projectsLabel,
}: {
  skill: SkillRecord;
  locale: "en" | "zh";
  projectsLabel: string;
}) {
  const t = useT();
  const desc = skillDisplayDescription(skill, locale);
  const cat = normalizeSkillCategory(skill.category);

  return (
    <li>
      <ControlCenterLink to="/agents/$skillId" params={{ skillId: skill.id }} className="dw-agents-skill-row">
        <span className="dw-agents-skill-row__icon">
          <Icon name="extension" size={18} />
        </span>
        <span className="dw-agents-skill-row__body min-w-0">
          <span className="flex flex-wrap items-center gap-1.5">
            <span className="dw-agents-skill-row__name">{skill.name}</span>
            <span className="text-[10px] px-1 py-0 rounded bg-surface-container-high text-secondary">
              {t(`agents.skillCategory.${cat}`)}
            </span>
          </span>
          {desc && <span className="dw-agents-skill-row__desc">{desc}</span>}
        </span>
        <span className="dw-agents-skill-row__meta shrink-0">
          <span className="tabular-nums">
            {skill.projects_count} {projectsLabel}
          </span>
          <Icon name="chevron_right" size={18} className="text-outline" />
        </span>
      </ControlCenterLink>
    </li>
  );
}
