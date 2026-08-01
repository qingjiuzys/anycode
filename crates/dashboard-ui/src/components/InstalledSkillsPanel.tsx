import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { api } from "@/api/client";
import type { SkillRecord } from "@/api/types";
import { EmptyState } from "@/components/EmptyState";
import { Icon } from "@/components/Icon";
import { useLocale, useT } from "@/i18n/context";
import {
  SKILL_CATEGORIES,
  filterSkillsByCategory,
  skillDisplayDescription,
  skillDisplayName,
  skillMatchesSearch,
  type SkillCategory,
} from "@/lib/skillCatalog";
import { skillIconMeta, skillIconToneClass } from "@/lib/skillIcons";

type Props = {
  skills: SkillRecord[];
  loading?: boolean;
  rescanPending?: boolean;
  onRescan?: () => void;
  rescanSuccess?: number;
  missingStarterCount?: number;
  onInstallStarter?: () => void;
  installStarterPending?: boolean;
  /** When true, omits outer panel chrome (for tabbed Agents page). */
  embedded?: boolean;
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
  embedded = false,
}: Props) {
  const t = useT();
  const locale = useLocale();
  const queryClient = useQueryClient();
  const [categoryFilter, setCategoryFilter] = useState<SkillCategory | "all">("all");
  const [search, setSearch] = useState("");
  const [pendingUninstallId, setPendingUninstallId] = useState<string | null>(null);
  const [uninstallError, setUninstallError] = useState<string | null>(null);
  const [lastUninstalled, setLastUninstalled] = useState<string | null>(null);

  const uninstall = useMutation({
    mutationFn: (id: string) => api.uninstallSkill(id),
    onMutate: () => setUninstallError(null),
    onSuccess: (result) => {
      setPendingUninstallId(null);
      setLastUninstalled(result.id);
      void queryClient.invalidateQueries({ queryKey: ["skills"] });
      void queryClient.invalidateQueries({ queryKey: ["skill-suggestions"] });
      void queryClient.invalidateQueries({ queryKey: ["overview"] });
    },
    onError: (e: Error) => setUninstallError(e.message),
  });

  const filtered = useMemo(() => {
    let list = skills.filter((s) => skillMatchesSearch(s, search));
    list = filterSkillsByCategory(list, categoryFilter);
    return list;
  }, [skills, search, categoryFilter]);

  const toolbar = (
    <header className={embedded ? "dw-agents-tab-toolbar" : "dw-agents-panel__head"}>
      <div>
        {!embedded && (
          <h2 id="agents-skills-heading" className="dw-agents-panel__title">
            {t("agents.skills")}
          </h2>
        )}
        {!loading && (
          <p className={`${embedded ? "text-[13px]" : "dw-agents-panel__sub"} text-secondary m-0`}>
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
  );

  const filters = skills.length > 0 && (
    <div
      className={
        embedded
          ? "pb-2 space-y-2"
          : "px-4 pt-3 pb-2 space-y-2 border-b border-outline-variant/40"
      }
    >
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
      <div
        className="flex flex-wrap items-center gap-1.5"
        role="tablist"
        aria-label={t("agents.skills")}
      >
        <CategoryTab
          active={categoryFilter === "all"}
          label={t("agents.skillCategory.all")}
          onClick={() => setCategoryFilter("all")}
        />
        {SKILL_CATEGORIES.map((cat) => (
          <CategoryTab
            key={cat}
            active={categoryFilter === cat}
            label={t(`agents.skillCategory.${cat}`)}
            onClick={() => setCategoryFilter(cat)}
          />
        ))}
      </div>
      <p className="text-[12px] text-secondary m-0">{t("agents.skillUninstallHint")}</p>
    </div>
  );

  const scrollBody = (
    <div
      className={
        embedded
          ? "dw-agents-panel__body--list dw-agents-panel__body--scroll min-h-0"
          : "dw-agents-panel__body dw-agents-panel__body--list dw-agents-panel__body--scroll"
      }
    >
      {rescanSuccess !== undefined && (
        <p className="dw-agents-toast m-0" role="status">
          <Icon name="check_circle" size={16} className="text-success" />
          {t("agents.rescanSuccess").replace("{n}", String(rescanSuccess))}
        </p>
      )}
      {lastUninstalled && (
        <p className="dw-agents-toast m-0" role="status">
          {t("agents.skillUninstallOk").replace("{name}", lastUninstalled)}
        </p>
      )}
      {uninstallError && (
        <p className="text-[13px] text-error m-0 px-1 py-2" role="alert">
          {uninstallError}
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
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3 p-1">
          {filtered.map((skill) => (
            <InstalledSkillCard
              key={skill.id}
              skill={skill}
              locale={locale}
              confirmUninstall={pendingUninstallId === skill.id}
              uninstalling={uninstall.isPending && pendingUninstallId === skill.id}
              onUninstall={() => {
                if (pendingUninstallId === skill.id) {
                  uninstall.mutate(skill.id);
                } else {
                  setPendingUninstallId(skill.id);
                }
              }}
              onCancelUninstall={() => setPendingUninstallId(null)}
            />
          ))}
        </div>
      )}
    </div>
  );

  if (embedded) {
    return (
      <div className="dw-agents-tab-panel dw-agents-tab-panel--skills flex flex-col min-h-0">
        {toolbar}
        {filters}
        {scrollBody}
      </div>
    );
  }

  return (
    <section className="dw-agents-panel dw-agents-panel--skills" aria-labelledby="agents-skills-heading">
      {toolbar}
      {filters}
      {scrollBody}
    </section>
  );
}

function CategoryTab({
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
      role="tab"
      aria-selected={active}
      className={`text-[13px] px-2.5 py-1 rounded-lg border transition-colors ${
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

function InstalledSkillCard({
  skill,
  locale,
  confirmUninstall,
  uninstalling,
  onUninstall,
  onCancelUninstall,
}: {
  skill: SkillRecord;
  locale: "en" | "zh";
  confirmUninstall: boolean;
  uninstalling: boolean;
  onUninstall: () => void;
  onCancelUninstall: () => void;
}) {
  const t = useT();
  const desc = skillDisplayDescription(skill, locale);
  const displayName = skillDisplayName(skill, locale);
  const { icon, tone } = skillIconMeta(skill);

  return (
    <div className="flex flex-col gap-3 p-4 rounded-xl border border-outline-variant bg-surface-container-lowest h-full min-h-[10.5rem]">
      <ControlCenterLink
        to="/agents/$skillId"
        params={{ skillId: skill.id }}
        className="flex items-start gap-3 min-w-0 no-underline text-inherit flex-1"
      >
        <span className={`dw-agents-skill-row__icon shrink-0 ${skillIconToneClass(tone)}`}>
          <Icon name={icon} size={20} />
        </span>
        <span className="flex flex-col gap-1 min-w-0 flex-1">
          <span className="text-sm font-semibold text-on-surface line-clamp-1">{displayName}</span>
          {desc && (
            <span className="text-[13px] text-secondary line-clamp-2 leading-relaxed">{desc}</span>
          )}
        </span>
      </ControlCenterLink>
      <div className="flex items-center gap-1.5 mt-auto">
        {confirmUninstall ? (
          <>
            <button
              type="button"
              className="dw-btn-ghost text-[12px] text-error px-2 flex-1"
              disabled={uninstalling}
              onClick={onUninstall}
            >
              {uninstalling ? t("common.loading") : t("agents.skillUninstallConfirm")}
            </button>
            <button
              type="button"
              className="dw-btn-ghost text-[12px] px-2"
              disabled={uninstalling}
              onClick={onCancelUninstall}
            >
              {t("common.cancel")}
            </button>
          </>
        ) : (
          <button
            type="button"
            className="dw-btn-ghost text-[12px] text-error px-2 w-full"
            onClick={onUninstall}
          >
            <Icon name="delete" size={14} className="inline mr-1" />
            {t("agents.skillUninstall")}
          </button>
        )}
      </div>
    </div>
  );
}
