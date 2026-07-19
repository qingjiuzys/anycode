import { useMutation } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import type { ConfiguredModel, ModelsRegistryView } from "@/api/types";
import { api } from "@/api/client";
import { ListPaginationBar } from "@/components/ui/ListPaginationBar";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

type Props = {
  items: ConfiguredModel[];
  registry?: ModelsRegistryView;
  onEdit: (item: ConfiguredModel) => void;
  onDelete: (id: string) => void;
  onRefresh: () => void;
};

const PAGE_SIZES = [5, 10, 20];

export function ConfiguredModelsList({ items, registry, onEdit, onDelete, onRefresh }: Props) {
  const t = useT();
  const active = registry?.active ?? {};
  const [page, setPage] = useState(0);
  const [pageSize, setPageSize] = useState(PAGE_SIZES[0]!);

  const test = useMutation({
    mutationFn: ({ id, cap }: { id: string; cap: string }) =>
      api.testModel(id, { capability: cap }),
  });

  const enable = useMutation({
    mutationFn: ({ id, cap }: { id: string; cap: string }) => api.enableModel(id, [cap]),
    onSuccess: onRefresh,
  });

  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  const pageItems = useMemo(
    () => items.slice(page * pageSize, page * pageSize + pageSize),
    [items, page, pageSize],
  );

  useEffect(() => {
    setPage(0);
  }, [pageSize, items.length]);

  useEffect(() => {
    if (page > 0 && page >= pageCount) setPage(Math.max(0, pageCount - 1));
  }, [page, pageCount]);

  return (
    <SectionCard title={t("settings.model.configuredTitle")} noPadding>
      {items.length === 0 ? (
        <p className="text-sm text-secondary m-0 p-4">{t("settings.model.configuredEmpty")}</p>
      ) : (
        <>
          <div className="space-y-3 p-4">
            {pageItems.map((item) => {
              const activeCaps = Object.entries(active)
                .filter(([, mid]) => mid === item.id)
                .map(([cap]) => cap);
              return (
                <div
                  key={item.id}
                  className="border border-outline-variant/60 rounded-lg p-4 flex flex-col gap-3 bg-surface-container-lowest"
                >
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="font-medium truncate flex items-center gap-2 text-on-surface">
                        <span className="truncate">{item.display_name ?? item.id}</span>
                        {item.source === "cloud" && (
                          <span className="dw-model-picker__cloud-badge">{t("modelPicker.cloudBadge")}</span>
                        )}
                      </div>
                      <div className="text-sm font-code text-secondary truncate">
                        {item.provider} / {item.model}
                      </div>
                    </div>
                    <div className="flex flex-wrap gap-1">
                      {item.capabilities.map((cap) => (
                        <span
                          key={cap}
                          className="text-[10px] uppercase tracking-wide px-2 py-0.5 rounded-md bg-surface-container-high text-secondary"
                        >
                          {cap}
                        </span>
                      ))}
                      {!item.enabled && <StatusBadge status="cancelled" />}
                    </div>
                  </div>

                  {activeCaps.length > 0 && (
                    <p className="text-xs text-secondary m-0">
                      {t("settings.model.activeFor")}: {activeCaps.join(", ")}
                    </p>
                  )}

                  <div className="flex flex-wrap gap-2">
                    {item.capabilities.map((cap) => (
                      <button
                        key={`enable-${cap}`}
                        type="button"
                        className="dw-btn-secondary text-xs"
                        disabled={enable.isPending}
                        onClick={() => enable.mutate({ id: item.id, cap })}
                      >
                        {activeCaps.includes(cap)
                          ? t("settings.model.enabled")
                          : t("settings.model.enableCap").replace("{cap}", cap)}
                      </button>
                    ))}
                    <button
                      type="button"
                      className="dw-btn-secondary text-xs"
                      disabled={test.isPending}
                      onClick={() =>
                        test.mutate({
                          id: item.id,
                          cap: item.capabilities[0] ?? "chat",
                        })
                      }
                    >
                      {test.isPending ? t("common.loading") : t("settings.model.testConnection")}
                    </button>
                    <button type="button" className="dw-btn-secondary text-xs" onClick={() => onEdit(item)}>
                      {t("settings.runtime.editModel")}
                    </button>
                    <button
                      type="button"
                      className="dw-btn-ghost text-xs"
                      onClick={() => onDelete(item.id)}
                    >
                      {t("common.delete")}
                    </button>
                  </div>

                  {test.isSuccess && test.variables?.id === item.id && test.data?.ok && (
                    <p className="text-xs text-secondary m-0">{test.data.message}</p>
                  )}
                  {test.isSuccess && test.variables?.id === item.id && test.data && !test.data.ok && (
                    <div className="dw-alert-error text-xs">{test.data.error}</div>
                  )}
                </div>
              );
            })}
          </div>
          <ListPaginationBar
            page={page}
            pageCount={pageCount}
            pageSize={pageSize}
            pageSizeOptions={PAGE_SIZES}
            total={items.length}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
          />
        </>
      )}
    </SectionCard>
  );
}
