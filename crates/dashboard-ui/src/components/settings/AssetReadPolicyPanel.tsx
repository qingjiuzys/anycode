import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

const RULE_KEYS = [
  "settings.assetPolicy.rule1",
  "settings.assetPolicy.rule2",
  "settings.assetPolicy.rule3",
  "settings.assetPolicy.rule4",
] as const;

export function AssetReadPolicyPanel() {
  const t = useT();
  const qc = useQueryClient();
  const prefs = useQuery({ queryKey: ["dashboard-preferences"], queryFn: api.dashboardPreferences });
  const runtime = useQuery({ queryKey: ["runtime-settings"], queryFn: api.runtimeSettings });

  const view = prefs.data?.preferences;
  const src = view?.saved ?? view?.active;
  const strict = src?.asset_read_strict ?? runtime.data?.runtime.asset_read_strict ?? false;

  const toggle = useMutation({
    mutationFn: (next: boolean) => {
      if (!src) throw new Error("preferences not loaded");
      return api.saveDashboardPreferences({
        host: src.host,
        port: src.port,
        db_path: src.db_path,
        asset_read_strict: next,
        model_fallback_provider: src.model_fallback_provider,
        model_fallback_model: src.model_fallback_model,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["dashboard-preferences"] });
      qc.invalidateQueries({ queryKey: ["runtime-settings"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const policy = runtime.data?.runtime.asset_read_policy;

  return (
    <SectionCard title={t("settings.assetPolicy.title")}>
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4">
        <label className="inline-flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            className="accent-primary"
            checked={strict}
            disabled={!src || toggle.isPending}
            onChange={(e) => toggle.mutate(e.target.checked)}
          />
          {t("settings.assetPolicy.strictMode")}
        </label>
        <StatusBadge status={strict ? "warn" : "ok"} />
      </div>

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
      {toggle.isError && (
        <div className="dw-alert-error mt-3">{(toggle.error as Error).message}</div>
      )}
    </SectionCard>
  );
}
