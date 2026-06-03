import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CatalogProviderRow, LlmConfigView, ModelCatalog } from "@/api/types";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

function catalogModelsForProvider(catalog: ModelCatalog | undefined, provider: string) {
  const id = provider.trim().toLowerCase();
  if (id === "z.ai" || id === "zai" || id === "bigmodel") {
    return catalog?.zai_models ?? [];
  }
  if (id === "google" || id === "gemini") {
    return catalog?.google_models ?? [];
  }
  return [];
}

function providerNeedsBaseUrl(entry?: CatalogProviderRow) {
  if (!entry) return false;
  return entry.id === "custom" || Boolean(entry.suggested_openai_base);
}

function providerNeedsPlan(provider: string) {
  const id = provider.trim().toLowerCase();
  return id === "z.ai" || id === "zai" || id === "bigmodel";
}

export function ModelSettingsPanel() {
  const t = useT();
  const qc = useQueryClient();

  const catalog = useQuery({
    queryKey: ["model-catalog"],
    queryFn: () => api.modelCatalog(),
  });

  const llm = useQuery({
    queryKey: ["llm-config"],
    queryFn: () => api.getLlmConfig(),
  });

  const [provider, setProvider] = useState("");
  const [model, setModel] = useState("");
  const [plan, setPlan] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [fallbackProvider, setFallbackProvider] = useState("");
  const [fallbackModel, setFallbackModel] = useState("");
  const [fallbackOn, setFallbackOn] = useState("geo");

  useEffect(() => {
    if (!llm.data) return;
    const cfg = llm.data;
    setProvider(cfg.provider ?? "");
    setModel(cfg.model ?? "");
    setPlan(cfg.plan ?? "");
    setBaseUrl(cfg.base_url ?? "");
    setApiKey("");
    setFallbackProvider(cfg.model_fallback?.provider ?? "");
    setFallbackModel(cfg.model_fallback?.model ?? "");
    setFallbackOn(cfg.model_fallback?.on ?? "geo");
  }, [llm.data]);

  const providerEntry = useMemo(
    () => catalog.data?.providers.find((p) => p.id === provider),
    [catalog.data?.providers, provider],
  );

  const modelOptions = useMemo(
    () => catalogModelsForProvider(catalog.data, provider),
    [catalog.data, provider],
  );

  const showPlan = providerNeedsPlan(provider);
  const showBaseUrl = providerNeedsBaseUrl(providerEntry);

  const save = useMutation({
    mutationFn: () =>
      api.putLlmConfig({
        provider: provider.trim() || undefined,
        model: model.trim() || undefined,
        plan: showPlan ? plan.trim() || undefined : plan.trim() || undefined,
        base_url: showBaseUrl ? baseUrl.trim() || undefined : baseUrl.trim() || undefined,
        api_key: apiKey.trim() || undefined,
        fallback_provider: fallbackProvider.trim() || undefined,
        fallback_model: fallbackModel.trim() || undefined,
        fallback_on: fallbackOn || undefined,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["llm-config"] });
      qc.invalidateQueries({ queryKey: ["runtime-settings"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
      setApiKey("");
    },
  });

  const test = useMutation({
    mutationFn: () => api.testLlm("chat"),
  });

  if (llm.isLoading || catalog.isLoading) {
    return (
      <SectionCard title={t("settings.model.chatTitle")}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </SectionCard>
    );
  }

  if (llm.isError) {
    return (
      <SectionCard title={t("settings.model.chatTitle")}>
        <div className="dw-alert-error">{(llm.error as Error).message}</div>
      </SectionCard>
    );
  }

  const cfg = llm.data as LlmConfigView;

  return (
    <SectionCard title={t("settings.model.chatTitle")}>
      {!cfg.config_present && (
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.runtime.configMissing")}</p>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.model.provider")}</span>
          <select
            className="dw-input font-code"
            value={provider}
            onChange={(e) => {
              setProvider(e.target.value);
              const entry = catalog.data?.providers.find((p) => p.id === e.target.value);
              if (entry?.suggested_openai_base && !baseUrl) {
                setBaseUrl(entry.suggested_openai_base);
              }
            }}
          >
            <option value="">{t("settings.model.providerPlaceholder")}</option>
            {(catalog.data?.providers ?? []).map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
                {p.placeholder_only ? ` (${t("settings.model.placeholderOnly")})` : ""}
              </option>
            ))}
          </select>
          {providerEntry?.hint && (
            <span className="text-xs text-secondary">{providerEntry.hint}</span>
          )}
        </label>

        <label className="flex flex-col gap-1 text-sm">
          <span className="text-secondary font-medium">{t("settings.model.model")}</span>
          {modelOptions.length > 0 ? (
            <select
              className="dw-input font-code"
              value={model}
              onChange={(e) => setModel(e.target.value)}
            >
              <option value="">{t("settings.model.modelPlaceholder")}</option>
              {modelOptions.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.label}
                </option>
              ))}
            </select>
          ) : (
            <input
              className="dw-input font-code"
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder={t("settings.model.modelFreeformPlaceholder")}
            />
          )}
        </label>

        {showPlan && (
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("settings.model.plan")}</span>
            <select className="dw-input font-code" value={plan} onChange={(e) => setPlan(e.target.value)}>
              <option value="">{t("settings.model.planPlaceholder")}</option>
              {(catalog.data?.zai_auth_methods ?? []).map((m) => (
                <option key={m.plan} value={m.plan}>
                  {m.label}
                </option>
              ))}
            </select>
          </label>
        )}

        {showBaseUrl && (
          <label className="flex flex-col gap-1 text-sm sm:col-span-2">
            <span className="text-secondary font-medium">{t("settings.model.baseUrl")}</span>
            <input
              className="dw-input font-code"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder={providerEntry?.suggested_openai_base ?? "https://"}
            />
          </label>
        )}

        <label className="flex flex-col gap-1 text-sm sm:col-span-2">
          <span className="text-secondary font-medium">{t("settings.model.apiKey")}</span>
          <input
            type="password"
            className="dw-input font-code"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder={
              cfg.api_key.configured
                ? `${t("settings.model.apiKeyConfigured")} (${cfg.api_key.preview ?? "***"})`
                : t("settings.model.apiKeyPlaceholder")
            }
            autoComplete="off"
          />
        </label>
      </div>

      <div className="border border-outline-variant rounded-lg p-4 mb-4">
        <h4 className="text-sm font-medium m-0 mb-3">{t("settings.model.fallbackTitle")}</h4>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("settings.runtime.fallbackProvider")}</span>
            <select
              className="dw-input font-code"
              value={fallbackProvider}
              onChange={(e) => setFallbackProvider(e.target.value)}
            >
              <option value="">{t("settings.model.providerPlaceholder")}</option>
              {(catalog.data?.providers ?? []).map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("settings.runtime.fallbackModel")}</span>
            <input
              className="dw-input font-code"
              value={fallbackModel}
              onChange={(e) => setFallbackModel(e.target.value)}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("settings.model.fallbackOn")}</span>
            <select
              className="dw-input font-code"
              value={fallbackOn}
              onChange={(e) => setFallbackOn(e.target.value)}
            >
              <option value="geo">{t("settings.model.fallbackOnGeo")}</option>
              <option value="rate_limit">{t("settings.model.fallbackOnRateLimit")}</option>
              <option value="any_error">{t("settings.model.fallbackOnAnyError")}</option>
            </select>
          </label>
        </div>
        <p className="text-xs text-secondary m-0 mt-3">{t("settings.model.fallbackHint")}</p>
      </div>

      <p className="text-xs text-secondary m-0 mb-3">{t("settings.model.saveHint")}</p>

      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={save.isPending}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.loading") : t("settings.model.save")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={test.isPending}
          onClick={() => test.mutate()}
        >
          {test.isPending ? t("common.loading") : t("settings.model.testConnection")}
        </button>
      </div>

      {save.isSuccess && (
        <p className="text-sm text-secondary mt-2 m-0">{t("settings.model.saved")}</p>
      )}
      {save.isError && (
        <div className="dw-alert-error mt-2">{(save.error as Error).message}</div>
      )}
      {test.isSuccess && test.data?.ok && (
        <p className="text-sm text-secondary mt-2 m-0">
          {test.data.message ?? t("settings.model.testSuccess")}
        </p>
      )}
      {test.isError && (
        <div className="dw-alert-error mt-2">{(test.error as Error).message}</div>
      )}
      {test.isSuccess && test.data && !test.data.ok && (
        <div className="dw-alert-error mt-2">{test.data.error ?? t("settings.model.testFailed")}</div>
      )}
    </SectionCard>
  );
}
