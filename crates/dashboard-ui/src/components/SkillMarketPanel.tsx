import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { api } from "@/api/client";
import type { SkillMarketEntry } from "@/api/types";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { Icon } from "@/components/Icon";
import { SkillsImportDialog } from "@/components/SkillsImportDialog";
import { SectionCard } from "@/components/ui/SectionCard";
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

export function SkillMarketPanel() {
  const t = useT();
  const locale = useLocale();
  const queryClient = useQueryClient();
  const [categoryFilter, setCategoryFilter] = useState<SkillCategory | "all">("all");
  const [search, setSearch] = useState("");
  const [importOpen, setImportOpen] = useState(false);

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
    mutationFn: (source: string) => api.importSkill(source),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["skills"] });
      queryClient.invalidateQueries({ queryKey: ["skill-suggestions"] });
      queryClient.invalidateQueries({ queryKey: ["overview"] });
    },
  });

  const installedIds = useMemo(
    () => new Set((installed.data?.skills ?? []).map((s) => s.id)),
    [installed.data?.skills],
  );

  const entries = useMemo(() => {
    const raw = market.data?.market.entries ?? [];
    let list = raw.filter((e) => skillMatchesSearch(e, search));
    list = filterSkillsByCategory(list, categoryFilter);
    return list;
  }, [market.data?.market.entries, search, categoryFilter]);

  const visibleCategories = useMemo(
    () => categoriesWithEntries(market.data?.market.entries ?? []),
    [market.data?.market.entries],
  );

  const grouped = useMemo(() => groupSkillsByCategory(entries), [entries]);

  return (
    <>
      <SectionCard
        title={t("agents.skillMarketTitle")}
        action={
          <button
            type="button"
            className="dw-btn-secondary text-xs"
            onClick={() => setImportOpen(true)}
          >
            <Icon name="upload" size={14} className="inline mr-1" />
            {t("agents.skillMarketImport")}
          </button>
        }
      >
        <p className="text-xs text-secondary m-0 mb-3">{t("agents.skillMarketHint")}</p>

        <div className="rounded-lg border border-outline-variant/60 bg-surface-container-low px-3 py-2.5 mb-3 text-xs text-secondary">
          <p className="m-0 font-medium text-on-surface">{t("agents.skillMarketDiscoverTitle")}</p>
          <p className="m-0 mt-1">{t("agents.skillMarketDiscoverHint")}</p>
          <div className="flex flex-wrap items-center gap-2 mt-2">
            <ExternalNavLink
              href="https://skills.sh"
              className="inline-flex items-center gap-1 text-primary no-underline hover:underline"
            >
              skills.sh
              <Icon name="open_in_new" size={14} />
            </ExternalNavLink>
            <button
              type="button"
              className="dw-btn-ghost text-xs py-0.5"
              onClick={() => setImportOpen(true)}
            >
              {t("agents.skillMarketImport")}
            </button>
          </div>
          <p className="m-0 mt-2 font-code text-[10px] text-outline break-all">
            {t("agents.skillMarketImportExample")}
          </p>
        </div>

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

        {market.isLoading && (
          <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
        )}
        {market.isError && (
          <p className="text-sm text-error m-0">{(market.error as Error).message}</p>
        )}
        {!market.isLoading && entries.length === 0 && (
          <p className="text-sm text-secondary m-0">{t("agents.skillMarketEmpty")}</p>
        )}

        <div className="space-y-4">
          {grouped.map((group) => (
            <div key={group.category}>
              <h4 className="text-xs font-semibold uppercase tracking-wide text-secondary m-0 mb-2 flex items-center gap-2">
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
                    installing={install.isPending}
                    onInstall={() => install.mutate(entry.source)}
                    t={t}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>

        {install.isSuccess && (
          <p className="text-xs text-secondary mt-3 m-0">
            {t("agents.skillMarketInstalled")}: {install.data?.id}
          </p>
        )}
        {install.isError && (
          <p className="text-xs text-error mt-3 m-0">{(install.error as Error).message}</p>
        )}
      </SectionCard>

      <SkillsImportDialog open={importOpen} onClose={() => setImportOpen(false)} />
    </>
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
      className={`text-xs px-2.5 py-1 rounded-full border transition-colors ${
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
  const cat = normalizeSkillCategory(entry.category);

  return (
    <div className="flex flex-col gap-2 p-3 rounded-lg border border-outline-variant bg-surface-container-lowest h-full">
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-sm font-medium text-on-surface">{entry.name}</span>
        <span className="text-[10px] px-1.5 py-0.5 rounded bg-surface-container-high text-secondary">
          {t(`agents.skillCategory.${cat}`)}
        </span>
        <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-secondary/15 text-secondary">
          {t("agents.skillMarketBadgeAnycode")}
        </span>
      </div>
      <p className="text-xs text-secondary m-0 line-clamp-2 flex-1">{desc || entry.description}</p>
      {locale === "en" && (
        <p className="text-[10px] text-outline font-code m-0 truncate" title={entry.source}>
          {entry.source}
        </p>
      )}
      <button
        type="button"
        className={`text-xs shrink-0 w-full ${installed ? "dw-btn-ghost" : "dw-btn-secondary"}`}
        disabled={installed || installing}
        title={locale === "zh" ? entry.source : undefined}
        onClick={onInstall}
      >
        <Icon name={installed ? "check_circle" : "download"} size={14} className="inline mr-1" />
        {installed ? t("agents.skillMarketAlreadyInstalled") : t("agents.skillMarketInstall")}
      </button>
    </div>
  );
}
