import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { get, put } from "@/api/http";
import { ListPaginationBar } from "@/components/ui/ListPaginationBar";
import { SectionCard } from "@/components/ui/SectionCard";
import { useI18n, useT } from "@/i18n/context";
import {
  isBuiltinPlugin,
  pluginDisplayDescription,
  pluginDisplayName,
} from "@/lib/pluginCatalog";

type PluginRow = {
  id: string;
  name: string;
  version: string;
  enabled: boolean;
  priority: number;
  tools: string[];
  overlay_preview?: string | null;
};

const PAGE_SIZES = [10, 20, 50];

export function SettingsPluginsSection() {
  const t = useT();
  const { locale } = useI18n();
  const queryClient = useQueryClient();
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(PAGE_SIZES[0]!);

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

  const rows = plugins.data ?? [];
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const pageItems = useMemo(
    () => rows.slice(page * pageSize, page * pageSize + pageSize),
    [rows, page, pageSize],
  );

  useEffect(() => {
    setPage(0);
  }, [pageSize]);

  useEffect(() => {
    if (page > 0 && page >= pageCount) setPage(Math.max(0, pageCount - 1));
  }, [page, pageCount]);

  return (
    <section className="space-y-4">
      <div>
        <h2 className="text-base font-semibold m-0 text-on-surface">{t("settings.plugins.title")}</h2>
        <p className="text-sm text-secondary mt-1 mb-0">{t("settings.plugins.subtitle")}</p>
      </div>

      {plugins.isLoading && <p className="text-sm text-secondary m-0">{t("common.loading")}</p>}
      {plugins.error && (
        <p className="text-sm text-error m-0">{(plugins.error as Error).message}</p>
      )}

      <SectionCard noPadding>
        <div className="flex flex-col gap-3 p-4">
          {pageItems.map((p) => {
            const displayName = pluginDisplayName(p.id, p.name, t);
            const description = pluginDisplayDescription(p.id, t);
            const version = p.version || "1.0.0";
            const meta = t("settings.plugins.meta")
              .replace("{id}", p.id)
              .replace("{version}", version);
            const showOverlay =
              p.overlay_preview &&
              (!description || (locale === "en" && isBuiltinPlugin(p.id)));

            return (
              <div key={p.id} className="dw-settings-inline-card flex flex-col gap-2">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <p className="font-medium m-0 text-on-surface">{displayName}</p>
                      {isBuiltinPlugin(p.id) && (
                        <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-surface-container-high text-secondary">
                          {t("settings.plugins.builtinTag")}
                        </span>
                      )}
                    </div>
                    <p className="text-xs text-secondary m-0 mt-0.5">{meta}</p>
                    {description && (
                      <p className="text-sm text-secondary m-0 mt-2">{description}</p>
                    )}
                  </div>
                  <label className="flex items-center gap-2 text-sm shrink-0 cursor-pointer text-on-surface">
                    <input
                      type="checkbox"
                      checked={p.enabled}
                      disabled={toggle.isPending}
                      onChange={(e) =>
                        toggle.mutate({ id: p.id, enabled: e.target.checked })
                      }
                    />
                    <span>{p.enabled ? t("common.enabled") : t("common.disabled")}</span>
                  </label>
                </div>
                {showOverlay && (
                  <div className="flex flex-col gap-1">
                    <p className="text-xs text-secondary m-0">
                      {t("settings.plugins.overlayPreview")}
                    </p>
                    <pre className="text-xs bg-surface-container-high p-2 rounded-lg overflow-x-auto m-0 whitespace-pre-wrap text-on-surface">
                      {p.overlay_preview}
                    </pre>
                  </div>
                )}
              </div>
            );
          })}
          {rows.length === 0 && !plugins.isLoading && (
            <p className="text-sm text-secondary m-0">{t("settings.plugins.empty")}</p>
          )}
        </div>
        {rows.length > 0 && (
          <ListPaginationBar
            page={page}
            pageCount={pageCount}
            pageSize={pageSize}
            pageSizeOptions={PAGE_SIZES}
            total={rows.length}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
          />
        )}
      </SectionCard>
    </section>
  );
}
