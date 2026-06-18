import { useQuery } from "@tanstack/react-query";
import { Icon } from "@/components/Icon";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useI18n } from "@/i18n/context";
import { docsHomeUrl, helpGuideUrl } from "@/lib/docLinks";
import { api } from "@/api/client";
import {
  FEATURE_NAV,
  FEATURE_NAV_GROUPS,
  navCount,
  type FeatureNavItem,
} from "@/lib/featureNav";

type Props = {
  activePath: string;
  onSelect: (item: FeatureNavItem) => void;
  className?: string;
};

export function FeatureNav({ activePath, onSelect, className = "" }: Props) {
  const { t, locale } = useI18n();
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview });
  const ov = overview.data?.overview;

  const isActive = (to: string) =>
    to === "/" ? activePath === "/" : activePath === to || activePath.startsWith(`${to}/`);

  return (
    <nav className={`dw-feature-nav ${className}`.trim()}>
      {FEATURE_NAV_GROUPS.map((group) => {
        const items = FEATURE_NAV.filter((item) => item.group === group.id);
        if (items.length === 0) return null;
        return (
          <div key={group.id} className="dw-feature-nav-group">
            <div className="dw-feature-nav-group-label">{t(group.labelKey as "nav.home")}</div>
            {items.map((item) => {
              const count = navCount(item.countKey, ov);
              const active = isActive(item.to);
              return (
                <button
                  key={item.id}
                  type="button"
                  className={`dw-feature-nav-link${active ? " active" : ""}`}
                  onClick={() => onSelect(item)}
                >
                  <Icon name={item.icon} filled={active} size={18} />
                  <span className="flex-1 min-w-0 truncate text-left">{t(item.key as "nav.home")}</span>
                  {count != null && count > 0 && (
                    <span className="text-[10px] font-semibold tabular-nums px-1.5 py-0.5 rounded-full bg-surface-container-high text-secondary">
                      {count}
                    </span>
                  )}
                </button>
              );
            })}
          </div>
        );
      })}
      <div className="dw-feature-nav-group">
        <div className="dw-feature-nav-group-label">{t("controlCenter.groupExternal")}</div>
        <ExternalNavLink href={docsHomeUrl(locale)} className="dw-feature-nav-link no-underline">
          <Icon name="description" size={18} />
          <span className="flex-1 min-w-0 truncate">{t("nav.docs")}</span>
        </ExternalNavLink>
        <ExternalNavLink href={helpGuideUrl(locale)} className="dw-feature-nav-link no-underline">
          <Icon name="help_outline" size={18} />
          <span className="flex-1 min-w-0 truncate">{t("nav.help")}</span>
        </ExternalNavLink>
      </div>
    </nav>
  );
}
