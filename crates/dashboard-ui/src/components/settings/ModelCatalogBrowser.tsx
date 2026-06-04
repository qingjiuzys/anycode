import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { CatalogModelRow, ConfiguredModel, ModelCatalog } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

function catalogModelsForProvider(catalog: ModelCatalog | undefined, provider: string): CatalogModelRow[] {
  const id = provider.trim().toLowerCase();
  if (id === "z.ai" || id === "zai" || id === "bigmodel") return catalog?.zai_models ?? [];
  if (id === "google" || id === "gemini") return catalog?.google_models ?? [];
  if (id === "deepseek" || id === "deep-seek") {
    return (
      catalog?.deepseek_models ??
      catalog?.provider_models?.deepseek ??
      catalog?.provider_models?.[provider] ??
      []
    );
  }
  return catalog?.provider_models?.[id] ?? catalog?.provider_models?.[provider] ?? [];
}

type Props = {
  catalog?: ModelCatalog;
  onAdd: (draft: ConfiguredModel) => void;
};

export function ModelCatalogBrowser({ catalog, onAdd }: Props) {
  const t = useT();
  const qc = useQueryClient();
  const [provider, setProvider] = useState("deepseek");
  const [query, setQuery] = useState("");
  const [capFilter, setCapFilter] = useState("");

  const refresh = useMutation({
    mutationFn: () =>
      api.refreshModelCatalog({
        provider,
        base_url: catalog?.providers.find((p) => p.id === provider)?.suggested_openai_base ?? undefined,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["model-catalog"] });
    },
  });

  const models = useMemo(() => {
    const base = catalogModelsForProvider(catalog, provider);
    return base.filter((m) => {
      const hay = `${m.id} ${m.label} ${m.description ?? ""}`.toLowerCase();
      if (query && !hay.includes(query.toLowerCase())) return false;
      if (capFilter && !(m.capabilities ?? ["chat"]).includes(capFilter)) return false;
      return true;
    });
  }, [catalog, provider, query, capFilter]);

  const meta = catalog?.cache_meta?.[provider];

  return (
    <SectionCard title={t("settings.model.catalogTitle")}>
      <div className="flex flex-wrap gap-2 mb-3">
        <select className="dw-input font-code" value={provider} onChange={(e) => setProvider(e.target.value)}>
          {(catalog?.providers ?? []).map((p) => (
            <option key={p.id} value={p.id}>
              {p.label}
            </option>
          ))}
        </select>
        <input
          className="dw-input flex-1 min-w-[12rem] font-code"
          placeholder={t("settings.model.catalogSearch")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <select className="dw-input" value={capFilter} onChange={(e) => setCapFilter(e.target.value)}>
          <option value="">{t("settings.model.allCapabilities")}</option>
          {(catalog?.capabilities ?? []).map((c) => (
            <option key={c.id} value={c.id}>
              {c.label ?? c.id}
            </option>
          ))}
        </select>
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={refresh.isPending}
          onClick={() => refresh.mutate()}
        >
          {refresh.isPending ? t("common.loading") : t("settings.model.refreshCatalog")}
        </button>
      </div>
      {meta && (
        <p className="text-xs text-secondary m-0 mb-3">
          {t("settings.model.catalogMeta")
            .replace("{source}", meta.source)
            .replace("{at}", meta.last_refreshed_at ?? "—")}
          {meta.offline_cache_used ? ` · ${t("settings.model.offlineCache")}` : ""}
          {meta.refresh_error ? ` · ${meta.refresh_error}` : ""}
        </p>
      )}
      <div className="max-h-48 overflow-y-auto border border-outline-variant rounded-lg divide-y divide-outline-variant">
        {models.length === 0 ? (
          <p className="text-sm text-secondary p-3 m-0">{t("settings.model.catalogEmpty")}</p>
        ) : (
          models.map((m) => (
            <div key={m.id} className="flex items-center justify-between gap-2 p-2 text-sm">
              <div className="min-w-0">
                <div className="font-code truncate">{m.id}</div>
                {m.description && <div className="text-xs text-secondary truncate">{m.description}</div>}
              </div>
              <button
                type="button"
                className="dw-btn-secondary shrink-0"
                onClick={() =>
                  onAdd({
                    id: `${provider.replace(/\./g, "-")}-${m.id}`.toLowerCase(),
                    provider,
                    model: m.id,
                    display_name: m.label,
                    capabilities: capFilter ? [capFilter] : ["chat"],
                    enabled: true,
                    source: "catalog",
                  })
                }
              >
                {t("settings.model.addFromCatalog")}
              </button>
            </div>
          ))
        )}
      </div>
    </SectionCard>
  );
}
