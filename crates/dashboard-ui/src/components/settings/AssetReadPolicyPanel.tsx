import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useDashboardPreferences } from "@/hooks/useDashboardPreferences";
import { useT } from "@/i18n/context";
import { useQuery } from "@tanstack/react-query";

const RULE_KEYS = [
  "settings.assetPolicy.rule1",
  "settings.assetPolicy.rule2",
  "settings.assetPolicy.rule3",
  "settings.assetPolicy.rule4",
] as const;

export function AssetReadPolicyPanel() {
  const t = useT();
  const { src, save } = useDashboardPreferences();
  const runtime = useQuery({ queryKey: ["runtime-settings"], queryFn: api.runtimeSettings });

  const strict = src?.asset_read_strict ?? runtime.data?.runtime.asset_read_strict ?? false;
  const policy = runtime.data?.runtime.asset_read_policy;

  return (
    <SectionCard title={t("settings.assetPolicy.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.userPrefs.assetHint")}</p>
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
        <label className="inline-flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            className="accent-primary"
            checked={strict}
            disabled={!src || save.isPending}
            onChange={(e) => save.mutate({ asset_read_strict: e.target.checked })}
          />
          {t("settings.assetPolicy.strictMode")}
        </label>
        <StatusBadge status={strict ? "warn" : "ok"} />
      </div>
      {save.isSuccess && (
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.userPrefs.saved")}</p>
      )}

      <h4 className="text-xs font-semibold text-on-surface uppercase tracking-wide m-0 mb-2">
        {t("settings.assetPolicy.summary")}
      </h4>
      <p className="text-sm text-secondary m-0 mb-4">
        {policy?.summary ?? t("settings.assetPolicy.summaryText")}
      </p>
      <h4 className="text-xs font-semibold text-on-surface uppercase tracking-wide m-0 mb-2">
        {t("settings.assetPolicy.rules")}
      </h4>
      <ul className="m-0 pl-5 text-sm text-secondary space-y-2">
        {(policy?.rules ?? RULE_KEYS.map((k) => t(k))).map((rule) => (
          <li key={rule}>{rule}</li>
        ))}
      </ul>
      {save.isError && (
        <div className="dw-alert-error mt-3">{(save.error as Error).message}</div>
      )}
    </SectionCard>
  );
}
