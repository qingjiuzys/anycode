import { CopyButton } from "@/components/ui/CopyButton";
import { SectionCard } from "@/components/ui/SectionCard";
import { useDashboardPreferences } from "@/hooks/useDashboardPreferences";
import { useT } from "@/i18n/context";
import { useState, useEffect } from "react";

export function DashboardPreferencesForm() {
  const t = useT();
  const { view, src, save } = useDashboardPreferences();

  const [host, setHost] = useState("127.0.0.1");
  const [port, setPort] = useState("43180");
  const [dbPath, setDbPath] = useState("");

  useEffect(() => {
    if (src) {
      setHost(src.host);
      setPort(String(src.port));
      setDbPath(src.db_path);
    }
  }, [src]);

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
          <input
            className="dw-input font-code text-xs"
            value={dbPath}
            onChange={(e) => setDbPath(e.target.value)}
          />
        </label>
      </div>

      <div className="flex flex-wrap items-center gap-2 mb-3">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={save.isPending || !src}
          onClick={() =>
            save.mutate({
              host: host.trim(),
              port: Number(port),
              db_path: dbPath.trim(),
            })
          }
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
    </SectionCard>
  );
}
