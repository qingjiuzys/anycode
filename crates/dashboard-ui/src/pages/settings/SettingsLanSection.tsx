import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SettingsLanSection() {
  const t = useT();
  const qc = useQueryClient();
  const settingsQuery = useQuery({
    queryKey: ["lan", "settings"],
    queryFn: () => api.getSettings(),
  });
  const settings = settingsQuery.data?.settings;

  const saveMut = useMutation({
    mutationFn: (body: {
      discovery_enabled?: boolean;
      display_name?: string;
      lan_port?: number;
      max_bundle_mb?: number;
    }) => api.patchSettings(body),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ["lan"] }),
  });

  return (
    <SectionCard title={t("settings.lan.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.lan.hint")}</p>
      {settingsQuery.isLoading ? (
        <p className="text-sm m-0">{t("common.loading")}</p>
      ) : settings ? (
        <form
          className="space-y-4 max-w-lg"
          onSubmit={(e) => {
            e.preventDefault();
            const fd = new FormData(e.currentTarget);
            void saveMut.mutateAsync({
              discovery_enabled: fd.get("discovery_enabled") === "on",
              display_name: String(fd.get("display_name") ?? ""),
              lan_port: Number(fd.get("lan_port") ?? settings.lan_port),
              max_bundle_mb: Number(fd.get("max_bundle_mb") ?? settings.max_bundle_mb),
            });
          }}
        >
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              name="discovery_enabled"
              defaultChecked={settings.discovery_enabled}
            />
            {t("settings.lan.discoveryEnabled")}
          </label>
          <label className="block text-sm">
            <span className="text-secondary">{t("settings.lan.displayName")}</span>
            <input
              className="dw-input w-full mt-1"
              name="display_name"
              defaultValue={settings.display_name}
            />
          </label>
          <label className="block text-sm">
            <span className="text-secondary">{t("settings.lan.lanPort")}</span>
            <input
              className="dw-input w-full mt-1"
              name="lan_port"
              type="number"
              min={1024}
              max={65535}
              defaultValue={settings.lan_port}
            />
          </label>
          <label className="block text-sm">
            <span className="text-secondary">{t("settings.lan.maxBundleMb")}</span>
            <input
              className="dw-input w-full mt-1"
              name="max_bundle_mb"
              type="number"
              min={10}
              max={5000}
              defaultValue={settings.max_bundle_mb}
            />
          </label>
          <button type="submit" className="dw-btn dw-btn--primary" disabled={saveMut.isPending}>
            {saveMut.isPending ? t("common.saving") : t("common.save")}
          </button>
          {saveMut.isError ? (
            <p className="text-sm text-error m-0">{(saveMut.error as Error).message}</p>
          ) : null}
        </form>
      ) : null}
    </SectionCard>
  );
}
