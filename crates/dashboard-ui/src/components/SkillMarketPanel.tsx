import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { api } from "@/api/client";
import type { SkillMarketEntry } from "@/api/types";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useLocale, useT } from "@/i18n/context";
import {
  categoriesWithEntries,
  filterSkillsByCategory,
  groupSkillsByCategory,
  normalizeSkillCategory,
  skillDisplayDescription,
  skillDisplayName,
  skillMatchesSearch,
  type SkillCategory,
} from "@/lib/skillCatalog";
import { skillIconMeta, skillIconToneClass } from "@/lib/skillIcons";

type Props = {
  /** When true, renders without outer SectionCard (for tabbed Agents page). */
  embedded?: boolean;
};

type StoreSection = {
  key: "official" | "anycode";
  title: string;
  hint?: string;
  entries: SkillMarketEntry[];
};

export function SkillMarketPanel({ embedded = false }: Props) {
  const t = useT();
  const locale = useLocale();
  const queryClient = useQueryClient();
  const [categoryFilter, setCategoryFilter] = useState<SkillCategory | "all">("all");
  const [search, setSearch] = useState("");
  const [installingId, setInstallingId] = useState<string | null>(null);
  const [lastInstalledId, setLastInstalledId] = useState<string | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);

  const market = useQuery({
    queryKey: ["skill-market"],
    queryFn: api.skillMarket,
    staleTime: 60_000,
  });
  const installed = useQuery({
    queryKey: ["skills"],
    queryFn: () => api.skills(200),
    staleTime: 30_000,
  });

  const install = useMutation({
    mutationFn: (id: string) => api.installMarketSkill(id),
    onMutate: (id) => {
      setInstallingId(id);
      setInstallError(null);
      setLastInstalledId(null);
    },
    onSuccess: (result) => {
      setLastInstalledId(result.id);
      queryClient.invalidateQueries({ queryKey: ["skills"] });
      queryClient.invalidateQueries({ queryKey: ["skill-suggestions"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
    },
    onError: (error) => {
      setInstallError((error as Error).message);
    },
    onSettled: () => {
      setInstallingId(null);
    },
  });

  const installedIds = useMemo(
    () => new Set((installed.data?.skills ?? []).map((s) => s.id)),
    [installed.data?.skills],
  );

  const filteredEntries = useMemo(() => {
    const raw = market.data?.market.entries ?? [];
    let list = raw.filter((e) => skillMatchesSearch(e, search));
    list = filterSkillsByCategory(list, categoryFilter);
    return list;
  }, [market.data?.market.entries, search, categoryFilter]);

  const sections = useMemo((): StoreSection[] => {
    const official = filteredEntries.filter((e) => e.badge === "official");
    const anycode = filteredEntries.filter((e) => e.badge === "anycode");
    const out: StoreSection[] = [];
    if (official.length > 0) {
      out.push({
        key: "official",
        title: t("agents.skillMarketOfficialSection"),
        hint: t("agents.skillMarketOfficialHint"),
        entries: official,
      });
    }
    if (anycode.length > 0) {
      out.push({
        key: "anycode",
        title: t("agents.skillMarketAnycodeSection"),
        hint: t("agents.skillMarketAnycodeHint"),
        entries: anycode,
      });
    }
    return out;
  }, [filteredEntries, t]);

  const visibleCategories = useMemo(
    () => categoriesWithEntries(market.data?.market.entries ?? []),
    [market.data?.market.entries],
  );

  const body = (
    <>
      {!embedded && <p className="text-[13px] text-secondary m-0 mb-3">{t("agents.skillMarketHint")}</p>}

      <div className="flex flex-wrap gap-1.5 mb-3">
        <CategoryPill
          active={categoryFilter === "all"}
          label={t("agents.skillCategory.all")}
          onClick={() => setCategoryFilter("all")}
        />
        {visibleCategories.map((cat) => (
          <CategoryPill
            key={cat}
            active={categoryFilter === cat}
            label={t(`agents.skillCategory.${cat}`)}
            onClick={() => setCategoryFilter(cat)}
          />
        ))}
      </div>

      <div className="relative mb-4">
        <Icon
          name="search"
          size={16}
          className="absolute left-3 top-1/2 -translate-y-1/2 text-outline"
        />
        <input
          type="search"
          className="dw-input w-full pl-9"
          placeholder={t("agents.skillMarketSearch")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      {market.isLoading && <p className="text-sm text-secondary m-0">{t("common.loading")}</p>}
      {market.isError && (
        <p className="text-sm text-error m-0">{(market.error as Error).message}</p>
      )}
      {!market.isLoading && filteredEntries.length === 0 && (
        <p className="text-sm text-secondary m-0">{t("agents.skillMarketEmpty")}</p>
      )}

      <div className="space-y-6">
        {sections.map((section) => {
          const grouped = groupSkillsByCategory(section.entries);
          return (
            <section key={section.key} className="dw-skill-store-section">
              <div className="mb-3">
                <h3 className="text-sm font-semibold text-on-surface m-0">{section.title}</h3>
                {section.hint && (
                  <p className="text-[13px] text-secondary m-0 mt-1">{section.hint}</p>
                )}
              </div>
              <div className="space-y-4">
                {grouped.map((group) => (
                  <div key={`${section.key}-${group.category}`}>
                    <h4 className="text-[13px] font-semibold uppercase tracking-wide text-secondary m-0 mb-2 flex items-center gap-2">
                      {t(`agents.skillCategory.${group.category}`)}
                      <span className="font-normal tabular-nums text-outline">{group.items.length}</span>
                    </h4>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                      {group.items.map((entry) => (
                        <MarketCard
                          key={`${entry.badge}-${entry.id}`}
                          entry={entry}
                          locale={locale}
                          installed={installedIds.has(entry.id)}
                          installing={installingId === entry.id}
                          onInstall={() => install.mutate(entry.id)}
                          t={t}
                        />
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </section>
          );
        })}
      </div>

      {lastInstalledId && (
        <p className="dw-agents-toast m-0 mt-3" role="status">
          {t("agents.skillMarketInstallOk").replace("{name}", lastInstalledId)}
        </p>
      )}
      {installError && (
        <p className="text-[13px] text-error mt-3 m-0" role="alert">
          {installError}
        </p>
      )}
    </>
  );

  if (embedded) {
    return <div className="dw-agents-tab-panel skill-market-panel">{body}</div>;
  }

  return (
    <SectionCard title={t("agents.skillMarketTitle")}>
      {body}
    </SectionCard>
  );
}

function CategoryPill({
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
      className={`text-[13px] px-2.5 py-1 rounded-full border transition-colors ${
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

function marketBadgeMeta(entry: SkillMarketEntry, t: (key: string) => string) {
  if (entry.badge === "official") {
    return {
      label: t("agents.skillMarketBadgeOfficial"),
      className: "dw-skill-badge dw-skill-badge--official",
    };
  }
  return {
    label: t("agents.skillMarketBadgeAnycode"),
    className: "dw-skill-badge dw-skill-badge--anycode",
  };
}

function MarketCard({
  entry,
  locale,
  installed,
  installing,
  onInstall,
  t,
}: {
  entry: SkillMarketEntry;
  locale: "en" | "zh";
  installed: boolean;
  installing: boolean;
  onInstall: () => void;
  t: (key: string) => string;
}) {
  const desc = skillDisplayDescription(entry, locale);
  const displayName = skillDisplayName(entry, locale);
  const cat = normalizeSkillCategory(entry.category);
  const badge = marketBadgeMeta(entry, t);
  const { icon, tone } = skillIconMeta(entry);

  return (
    <div className="flex flex-col gap-3 p-4 rounded-xl border border-outline-variant bg-surface-container-lowest h-full">
      <div className="flex items-start gap-3 min-w-0">
        <span className={`dw-agents-skill-row__icon ${skillIconToneClass(tone)}`}>
          <Icon name={icon} size={20} />
        </span>
        <div className="flex flex-col gap-1.5 min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-semibold text-on-surface">{displayName}</span>
            <span className="text-[13px] px-1.5 py-0.5 rounded-md bg-surface-container-high text-secondary">
              {t(`agents.skillCategory.${cat}`)}
            </span>
            <span className={badge.className}>{badge.label}</span>
          </div>
          <p className="text-[13px] text-secondary m-0 line-clamp-2 leading-relaxed flex-1">
            {desc || entry.description}
          </p>
        </div>
      </div>
      <button
        type="button"
        className={`text-[13px] shrink-0 w-full ${installed ? "dw-btn-ghost" : "dw-btn-secondary"}`}
        disabled={installed || installing}
        onClick={onInstall}
      >
        <Icon
          name={installed ? "check_circle" : installing ? "hourglass_top" : "download"}
          size={14}
          className="inline mr-1"
        />
        {installed
          ? t("agents.skillMarketAlreadyInstalled")
          : installing
            ? t("agents.skillMarketInstalling")
            : t("agents.skillMarketInstall")}
      </button>
    </div>
  );
}
