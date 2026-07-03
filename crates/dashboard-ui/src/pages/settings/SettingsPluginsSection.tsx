import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { get, put } from "@/api/http";
import { useT } from "@/i18n/context";

type PluginRow = {
  id: string;
  name: string;
  version: string;
  enabled: boolean;
  priority: number;
  tools: string[];
  overlay_preview?: string | null;
};

export function SettingsPluginsSection() {
  const t = useT();
  const queryClient = useQueryClient();
  const plugins = useQuery({
    queryKey: ["plugins"],
    queryFn: async () => {
      const res = await get<{ plugins: PluginRow[] }>("/api/plugins");
      return res.plugins;
    },
  });

  const toggle = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      put(`/api/plugins/${encodeURIComponent(id)}`, { enabled }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["plugins"] });
    },
  });

  return (
    <section className="dw-settings-section">
      <h2 className="text-lg font-semibold m-0">{t("settings.plugins.title")}</h2>
      <p className="text-sm text-secondary mt-1 mb-4">{t("settings.plugins.subtitle")}</p>

      {plugins.isLoading && <p className="text-sm text-secondary">{t("common.loading")}</p>}
      {plugins.error && (
        <p className="text-sm text-error">{(plugins.error as Error).message}</p>
      )}

      <div className="flex flex-col gap-3">
        {(plugins.data ?? []).map((p) => (
          <div key={p.id} className="dw-card p-4 flex flex-col gap-2">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="font-medium m-0">{p.name}</p>
                <p className="text-xs text-secondary m-0">
                  {p.id} · v{p.version || "1.0.0"}
                </p>
              </div>
              <label className="flex items-center gap-2 text-sm shrink-0">
                <input
                  type="checkbox"
                  checked={p.enabled}
                  disabled={toggle.isPending}
                  onChange={(e) =>
                    toggle.mutate({ id: p.id, enabled: e.target.checked })
                  }
                />
                <span>{t("settings.plugins.enabled")}</span>
              </label>
            </div>
            {p.overlay_preview && (
              <pre className="text-xs bg-surface-container-high p-2 rounded-lg overflow-x-auto m-0 whitespace-pre-wrap">
                {p.overlay_preview}
              </pre>
            )}
          </div>
        ))}
        {(plugins.data ?? []).length === 0 && !plugins.isLoading && (
          <p className="text-sm text-secondary">{t("settings.plugins.empty")}</p>
        )}
      </div>
    </section>
  );
}
