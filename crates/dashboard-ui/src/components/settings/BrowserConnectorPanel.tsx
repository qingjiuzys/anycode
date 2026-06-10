import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function BrowserConnectorPanel() {
  const t = useT();
  const queryClient = useQueryClient();
  const status = useQuery({
    queryKey: ["browser-connector"],
    queryFn: api.browserConnector,
  });

  const toggle = useMutation({
    mutationFn: (enabled: boolean) => api.setBrowserConnector(enabled),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["browser-connector"] });
      void queryClient.invalidateQueries({ queryKey: ["doctor"] });
    },
  });

  const data = status.data;
  const enabled = data?.enabled ?? false;
  const bundled = data?.bundled ?? false;
  const chromiumReady = data?.chromium_ready ?? false;
  const busy = toggle.isPending || status.isLoading;

  return (
    <SectionCard title={t("settings.browserConnector.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.browserConnector.hint")}</p>

      <div className="flex flex-wrap items-center gap-3 mb-3">
        <button
          type="button"
          className={`inline-flex items-center gap-2 dw-btn-primary ${enabled ? "opacity-90" : ""}`}
          disabled={busy || !bundled}
          onClick={() => toggle.mutate(!enabled)}
        >
          <Icon name={enabled ? "verified" : "radar"} size={16} />
          {enabled
            ? t("settings.browserConnector.disable")
            : t("settings.browserConnector.enable")}
        </button>
        <span
          className={`text-xs font-medium px-2 py-1 rounded-full ${
            enabled && bundled && chromiumReady
              ? "bg-success-container text-success"
              : enabled
                ? "bg-warn-container text-warn"
                : "bg-surface-container-high text-secondary"
          }`}
        >
          {enabled
            ? bundled && chromiumReady
              ? t("settings.browserConnector.statusReady")
              : t("settings.browserConnector.statusIncomplete")
            : t("settings.browserConnector.statusOff")}
        </span>
      </div>

      {!bundled && (
        <p className="text-sm text-warn m-0 mb-2">{t("settings.browserConnector.notBundled")}</p>
      )}
      {bundled && data?.bundle_path && (
        <p className="text-xs text-secondary font-code m-0 break-all">{data.bundle_path}</p>
      )}
      {toggle.isError && (
        <p className="text-sm text-error m-0 mt-2">{t("settings.browserConnector.error")}</p>
      )}
      {enabled && (
        <p className="text-xs text-secondary m-0 mt-3">{t("settings.browserConnector.restartHint")}</p>
      )}
    </SectionCard>
  );
}
