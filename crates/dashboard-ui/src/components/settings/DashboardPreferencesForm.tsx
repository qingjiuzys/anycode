import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { CopyButton } from "@/components/ui/CopyButton";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { useState, useEffect } from "react";

export function DashboardPreferencesForm() {
  const t = useT();
  const qc = useQueryClient();
  const prefs = useQuery({ queryKey: ["dashboard-preferences"], queryFn: api.dashboardPreferences });
  const view = prefs.data?.preferences;

  const [host, setHost] = useState("127.0.0.1");
  const [port, setPort] = useState("43180");
  const [dbPath, setDbPath] = useState("");
  const [assetStrict, setAssetStrict] = useState(false);

  useEffect(() => {
    const src = view?.saved ?? view?.active;
    if (src) {
      setHost(src.host);
      setPort(String(src.port));
      setDbPath(src.db_path);
      setAssetStrict(Boolean(src.asset_read_strict));
    }
  }, [view?.saved, view?.active]);

  const save = useMutation({
    mutationFn: () =>
      api.saveDashboardPreferences({
        host: host.trim(),
        port: Number(port),
        db_path: dbPath.trim(),
        asset_read_strict: assetStrict,
        model_fallback_provider: view?.saved?.model_fallback_provider ?? view?.active?.model_fallback_provider,
        model_fallback_model: view?.saved?.model_fallback_model ?? view?.active?.model_fallback_model,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["dashboard-preferences"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  return (
    <SectionCard title={t("settings.prefs.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.prefs.hint")}</p>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.runtime.dashboardHost")}</span>
          <input className="dw-input font-code" value={host} onChange={(e) => setHost(e.target.value)} />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.runtime.dashboardPort")}</span>
          <input className="dw-input font-code" value={port} onChange={(e) => setPort(e.target.value)} />
        </label>
        <label className="flex flex-col gap-1 text-sm sm:col-span-2">
          <span className="text-secondary font-medium">{t("settings.database")}</span>
          <input className="dw-input font-code text-xs" value={dbPath} onChange={(e) => setDbPath(e.target.value)} />
        </label>
      </div>

      <div className="flex flex-wrap items-center gap-2 mb-3">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={save.isPending}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.loading") : t("settings.prefs.save")}
        </button>
        {save.isSuccess && (
          <span className="text-sm text-secondary">{t("settings.prefs.saved")}</span>
        )}
      </div>

      {save.isError && (
        <div className="dw-alert-error mb-3">{(save.error as Error).message}</div>
      )}

      {view?.restart_required && (
        <div className="dw-alert-warn mb-3">{t("settings.prefs.restartRequired")}</div>
      )}

      {view?.restart_command && (
        <div className="flex flex-wrap items-center gap-2 bg-surface-container-low border border-outline-variant rounded p-3">
          <code className="font-code text-xs flex-1 break-all">{view.restart_command}</code>
          <CopyButton text={view.restart_command} />
        </div>
      )}

      {view?.preferences_path && (
        <p className="text-xs text-secondary mt-3 m-0">
          {t("settings.prefs.file")}: <code className="font-code">{view.preferences_path}</code>
        </p>
      )}
    </SectionCard>
  );
}
