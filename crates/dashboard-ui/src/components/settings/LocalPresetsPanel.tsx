import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { ConfiguredModel, LocalMediaPreset, LocalPresetsView, ModelCatalog } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const CAP_ORDER = ["embedding", "vision", "chat", "stt", "tts"] as const;

function presetToConfigured(preset: LocalMediaPreset): ConfiguredModel {
  const extra_headers = preset.voice ? { voice: preset.voice } : null;
  return {
    id: preset.id,
    display_name: preset.label,
    provider: preset.provider,
    model: preset.model,
    capabilities: preset.capabilities,
    plan: null,
    base_url: preset.base_url ?? null,
    api_key: preset.mode === "builtin" ? "local" : preset.provider === "ollama" ? "ollama" : "local",
    api_key_ref: null,
    temperature: null,
    max_tokens: null,
    extra_headers,
    endpoint_overrides: null,
    enabled: true,
    tags: ["local"],
    source: "local_preset",
  };
}

type Props = {
  catalog?: ModelCatalog;
  existingIds: Set<string>;
};

export function LocalPresetsPanel({ catalog, existingIds }: Props) {
  const t = useT();
  const qc = useQueryClient();
  const local = catalog?.local_presets as LocalPresetsView | undefined;
  const presets = local?.presets ?? [];

  const refreshAll = () => {
    qc.invalidateQueries({ queryKey: ["models-registry"] });
    qc.invalidateQueries({ queryKey: ["llm-config"] });
    qc.invalidateQueries({ queryKey: ["runtime-settings"] });
  };

  const applyPreset = useMutation({
    mutationFn: async (preset: LocalMediaPreset) => {
      const item = presetToConfigured(preset);
      await api.putModelsRegistry({ items: [item] });
      for (const cap of preset.capabilities) {
        await api.enableModel(item.id, [cap]);
      }
    },
    onSuccess: refreshAll,
  });

  const applyBundle = useMutation({
    mutationFn: async (ids: string[]) => {
      for (const id of ids) {
        const preset = presets.find((p) => p.id === id);
        if (!preset) continue;
        const item = presetToConfigured(preset);
        await api.putModelsRegistry({ items: [item] });
        for (const cap of preset.capabilities) {
          await api.enableModel(item.id, [cap]);
        }
      }
    },
    onSuccess: refreshAll,
  });

  if (presets.length === 0) return null;

  const byCap = new Map<string, LocalMediaPreset[]>();
  for (const p of presets) {
    for (const cap of p.capabilities) {
      const list = byCap.get(cap) ?? [];
      list.push(p);
      byCap.set(cap, list);
    }
  }

  const bundleIds = (local?.lightweight_bundle ?? []).filter((id) =>
    presets.some((p) => p.id === id),
  );

  return (
    <SectionCard title={t("settings.model.localPresets.title")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.model.localPresets.hint")}</p>
      {bundleIds.length > 0 && (
        <button
          type="button"
          className="dw-btn-secondary mb-4 text-sm"
          disabled={applyBundle.isPending}
          onClick={() => applyBundle.mutate(bundleIds)}
        >
          {t("settings.model.localPresets.applyBundle")}
        </button>
      )}
      <div className="flex flex-col gap-4">
        {CAP_ORDER.filter((cap) => byCap.has(cap)).map((cap) => (
          <div key={cap}>
            <h4 className="text-xs font-medium uppercase tracking-wide text-secondary m-0 mb-2">
              {t(`settings.model.capabilities.${cap}` as "settings.model.capabilities.chat")}
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {(byCap.get(cap) ?? []).map((preset) => {
                const installed = existingIds.has(preset.id);
                const needsBuild = preset.mode === "builtin" && !preset.feature_available;
                return (
                  <div
                    key={preset.id}
                    className="border border-outline-variant rounded-lg p-3 flex flex-col gap-2"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <span className="text-sm font-medium m-0">{preset.label}</span>
                      <span
                        className={`text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded shrink-0 ${
                          preset.mode === "builtin"
                            ? "bg-primary/10 text-primary"
                            : "bg-secondary/10 text-secondary"
                        }`}
                      >
                        {preset.mode === "builtin"
                          ? t("settings.model.localPresets.modeBuiltin")
                          : t("settings.model.localPresets.modeExternal")}
                      </span>
                    </div>
                    <p className="text-xs text-secondary m-0">{preset.description}</p>
                    {preset.model_download_hint && (
                      <p className="text-[11px] font-code text-secondary m-0 truncate" title={preset.model_download_hint}>
                        {preset.model_download_hint}
                      </p>
                    )}
                    {needsBuild && (
                      <p className="text-xs text-warning m-0">
                        {t("settings.model.localPresets.needsFeature").replace(
                          "{feature}",
                          preset.required_feature ?? "media-local",
                        )}
                      </p>
                    )}
                    {preset.docs_url && (
                      <a
                        href={preset.docs_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-xs text-primary hover:underline"
                      >
                        {t("settings.model.localPresets.docsLink")}
                      </a>
                    )}
                    <button
                      type="button"
                      className="dw-btn-secondary text-xs mt-1 self-start"
                      disabled={applyPreset.isPending || installed}
                      onClick={() => applyPreset.mutate(preset)}
                    >
                      {installed
                        ? t("settings.model.localPresets.installed")
                        : t("settings.model.localPresets.addAndEnable")}
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </SectionCard>
  );
}
