import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { RuntimeSettings } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function ModelConfigForm({ runtime }: { runtime?: RuntimeSettings }) {
  const t = useT();
  const qc = useQueryClient();
  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const [fallbackProvider, setFallbackProvider] = useState("");
  const [fallbackModel, setFallbackModel] = useState("");

  useEffect(() => {
    if (runtime) {
      setProvider(runtime.global_provider ?? "");
      setModel(runtime.global_model ?? "");
      setFallbackProvider(runtime.fallback_provider ?? "");
      setFallbackModel(runtime.fallback_model ?? "");
    }
  }, [runtime]);

  const save = useMutation({
    mutationFn: () =>
      api.patchLlmConfig({
        provider: provider.trim() || undefined,
        model: model.trim() || undefined,
        fallback_provider: fallbackProvider.trim() || undefined,
        fallback_model: fallbackModel.trim() || undefined,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["runtime-settings"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  if (!runtime) return null;

  return (
    <SectionCard title={t("settings.runtime.editModel")}>
      {!runtime.config_present && (
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.runtime.configMissing")}</p>
      )}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.runtime.globalProvider")}</span>
          <input className="dw-input font-code" value={provider} onChange={(e) => setProvider(e.target.value)} />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.runtime.globalModel")}</span>
          <input className="dw-input font-code" value={model} onChange={(e) => setModel(e.target.value)} />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.runtime.fallbackProvider")}</span>
          <input
            className="dw-input font-code"
            value={fallbackProvider}
            onChange={(e) => setFallbackProvider(e.target.value)}
          />
        </label>
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.runtime.fallbackModel")}</span>
          <input
            className="dw-input font-code"
            value={fallbackModel}
            onChange={(e) => setFallbackModel(e.target.value)}
          />
        </label>
      </div>
      <p className="text-xs text-secondary m-0 mb-3">{t("settings.runtime.modelSaveHint")}</p>
      <button
        type="button"
        className="dw-btn-primary"
        disabled={save.isPending}
        onClick={() => save.mutate()}
      >
        {save.isPending ? t("common.loading") : t("settings.runtime.saveModel")}
      </button>
      {save.isSuccess && (
        <p className="text-sm text-secondary mt-2 m-0">{t("settings.runtime.modelSaved")}</p>
      )}
      {save.isError && (
        <div className="dw-alert-error mt-2">{(save.error as Error).message}</div>
      )}
    </SectionCard>
  );
}
