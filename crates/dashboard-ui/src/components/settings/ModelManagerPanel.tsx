import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ConfiguredModel, LlmConfigView } from "@/api/types";
import { api } from "@/api/client";
import { CapabilityActiveMatrix } from "@/components/settings/CapabilityActiveMatrix";
import { ConfiguredModelsList } from "@/components/settings/ConfiguredModelsList";
import { LocalPresetsPanel } from "@/components/settings/LocalPresetsPanel";
import { LocalRuntimePanel } from "@/components/settings/LocalRuntimePanel";
import { ModelCatalogBrowser } from "@/components/settings/ModelCatalogBrowser";
import { ModelEditorDrawer } from "@/components/settings/ModelEditorDrawer";
import { ModelSettingsPanel } from "@/components/settings/ModelSettingsPanel";
import { RoutingAgentsEditor } from "@/components/settings/RoutingAgentsEditor";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

type ModelTab = "active" | "routing" | "connection" | "library";

function maskToConfigured(
  items: Array<{
    id: string;
    display_name?: string | null;
    provider: string;
    model: string;
    capabilities: string[];
    enabled: boolean;
    source?: string | null;
  }>,
): ConfiguredModel[] {
  return items.map((item) => ({
    ...item,
    plan: null,
    base_url: null,
    api_key: null,
    api_key_ref: null,
    temperature: null,
    max_tokens: null,
    extra_headers: null,
    endpoint_overrides: null,
    tags: null,
  }));
}

function isMockModelProfile(provider: string, model: string): boolean {
  const p = provider.trim().toLowerCase();
  const m = model.trim().toLowerCase();
  return p === "mock" || m === "mock" || m.startsWith("mock/");
}

function GlobalChatSummary({
  llm,
  onEditConnection,
}: {
  llm?: LlmConfigView;
  onEditConnection: () => void;
}) {
  const t = useT();
  if (!llm) return null;

  const label =
    llm.provider && llm.model
      ? `${llm.provider} / ${llm.model}`
      : t("settings.model.notConfigured");

  return (
    <div className="dw-model-global-summary">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-xs font-medium text-secondary">{t("settings.model.globalChat")}</span>
          {llm.config_present ? <StatusBadge status="ok" /> : <StatusBadge status="pending" />}
        </div>
        <p className="text-sm font-code m-0 mt-1 truncate" title={label}>
          {label}
        </p>
        <p className="text-[11px] text-secondary m-0 mt-1 leading-snug">
          {t("settings.model.globalChatHint")}
        </p>
      </div>
      <button type="button" className="dw-btn-secondary text-xs shrink-0" onClick={onEditConnection}>
        <Icon name="tune" size={14} />
        {t("settings.model.editConnection")}
      </button>
    </div>
  );
}

export function ModelManagerPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [activeTab, setActiveTab] = useState<ModelTab>("active");
  const [editorOpen, setEditorOpen] = useState(false);
  const [draft, setDraft] = useState<ConfiguredModel | null>(null);

  const catalog = useQuery({
    queryKey: ["model-catalog"],
    queryFn: () => api.modelCatalog(),
  });

  const registryQuery = useQuery({
    queryKey: ["models-registry"],
    queryFn: () => api.getModelsRegistry(),
  });

  const llm = useQuery({
    queryKey: ["llm-config"],
    queryFn: () => api.getLlmConfig(),
  });

  const items: ConfiguredModel[] = useMemo(() => {
    const fromRegistry = registryQuery.data?.items ?? [];
    const source = fromRegistry.length > 0 ? fromRegistry : maskToConfigured(llm.data?.registry?.items ?? []);
    return source.filter((item) => !isMockModelProfile(item.provider, item.model));
  }, [registryQuery.data?.items, llm.data?.registry?.items]);

  const existingPresetIds = useMemo(
    () => new Set(items.map((i) => i.id)),
    [items],
  );

  const routingCount = Object.keys(llm.data?.routing_agents ?? {}).length;
  const activeCapCount = Object.keys(registryQuery.data?.active ?? {}).length;

  const refreshAll = () => {
    qc.invalidateQueries({ queryKey: ["models-registry"] });
    qc.invalidateQueries({ queryKey: ["llm-config"] });
    qc.invalidateQueries({ queryKey: ["runtime-settings"] });
  };

  const saveModel = useMutation({
    mutationFn: (item: ConfiguredModel) =>
      api.putModelsRegistry({ items: [item] }),
    onSuccess: () => {
      refreshAll();
      setEditorOpen(false);
      setDraft(null);
    },
  });

  const deleteModel = useMutation({
    mutationFn: (id: string) => api.putModelsRegistry({ delete_ids: [id] }),
    onSuccess: refreshAll,
  });

  const enableCap = useMutation({
    mutationFn: ({ id, cap }: { id: string; cap: string }) => api.enableModel(id, [cap]),
    onSuccess: refreshAll,
  });

  const testDraft = useMutation({
    mutationFn: (item: ConfiguredModel) =>
      api.testModel(item.id, {
        capability: item.capabilities[0] ?? "chat",
        draft: item,
      }),
  });

  const tabs: { id: ModelTab; label: string; badge?: string }[] = [
    { id: "active", label: t("settings.model.tabs.active"), badge: String(activeCapCount) },
    { id: "routing", label: t("settings.model.tabs.routing"), badge: routingCount > 0 ? String(routingCount) : undefined },
    { id: "connection", label: t("settings.model.tabs.connection") },
    { id: "library", label: t("settings.model.tabs.library"), badge: items.length > 0 ? String(items.length) : undefined },
  ];

  if (registryQuery.isLoading || catalog.isLoading) {
    return (
      <SectionCard title={t("settings.model.managerTitle")}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </SectionCard>
    );
  }

  return (
    <>
      <section className="dw-model-settings-shell" aria-label={t("settings.model.managerTitle")}>
        <header className="dw-model-settings-shell__head">
          <div className="min-w-0">
            <h2 className="text-base font-semibold m-0 text-on-surface">{t("settings.model.managerTitle")}</h2>
            <p className="text-xs text-secondary m-0 mt-1">{t("settings.model.managerHint")}</p>
          </div>
          {activeTab === "library" && (
            <button
              type="button"
              className="dw-btn-primary text-sm shrink-0"
              onClick={() => {
                setDraft({
                  id: `custom-${Date.now()}`,
                  provider: "custom",
                  model: "",
                  capabilities: ["chat"],
                  enabled: true,
                  source: "custom",
                });
                setEditorOpen(true);
              }}
            >
              <Icon name="add" size={16} />
              {t("settings.model.addCustom")}
            </button>
          )}
        </header>

        <div className="dw-agents-tabs" role="tablist" aria-label={t("settings.model.managerTitle")}>
          {tabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              id={`model-tab-${tab.id}`}
              aria-selected={activeTab === tab.id}
              aria-controls={`model-panel-${tab.id}`}
              className={`dw-agents-tabs__tab ${activeTab === tab.id ? "dw-agents-tabs__tab--active" : ""}`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
              {tab.badge != null && (
                <span className="ml-1.5 text-[10px] font-semibold tabular-nums text-secondary">
                  {tab.badge}
                </span>
              )}
            </button>
          ))}
        </div>

        <div className="dw-model-tab-content">
          {activeTab === "active" && (
            <div
              role="tabpanel"
              id="model-panel-active"
              aria-labelledby="model-tab-active"
              className="dw-model-tab-panel-wrap"
            >
              <GlobalChatSummary
                llm={llm.data}
                onEditConnection={() => setActiveTab("connection")}
              />
              <CapabilityActiveMatrix
                embedded
                registry={registryQuery.data}
                items={items}
                enabling={enableCap.isPending}
                onEnable={(id, cap) => enableCap.mutate({ id, cap })}
              />
            </div>
          )}

          {activeTab === "routing" && (
            <div
              role="tabpanel"
              id="model-panel-routing"
              aria-labelledby="model-tab-routing"
              className="dw-model-tab-panel-wrap"
            >
              <RoutingAgentsEditor embedded />
            </div>
          )}

          {activeTab === "connection" && (
            <div
              role="tabpanel"
              id="model-panel-connection"
              aria-labelledby="model-tab-connection"
              className="dw-model-tab-panel-wrap"
            >
              <ModelSettingsPanel embedded />
            </div>
          )}

          {activeTab === "library" && (
            <div
              role="tabpanel"
              id="model-panel-library"
              aria-labelledby="model-tab-library"
              className="dw-model-tab-panel-wrap dw-model-tab-panel-wrap--stack"
            >
              <LocalRuntimePanel />
              <LocalPresetsPanel catalog={catalog.data} existingIds={existingPresetIds} />
              <ModelCatalogBrowser
                catalog={catalog.data}
                onAdd={(item) => {
                  setDraft(item);
                  setEditorOpen(true);
                }}
              />
              <ConfiguredModelsList
                items={items}
                registry={registryQuery.data}
                onEdit={(item) => {
                  setDraft(item);
                  setEditorOpen(true);
                }}
                onDelete={(id) => deleteModel.mutate(id)}
                onRefresh={refreshAll}
              />
            </div>
          )}
        </div>
      </section>

      <ModelEditorDrawer
        open={editorOpen}
        draft={draft}
        providers={catalog.data?.providers ?? []}
        onClose={() => {
          setEditorOpen(false);
          setDraft(null);
        }}
        onSave={(item) => saveModel.mutate(item)}
        onTest={(item) => testDraft.mutate(item)}
        testing={testDraft.isPending}
      />
    </>
  );
}
