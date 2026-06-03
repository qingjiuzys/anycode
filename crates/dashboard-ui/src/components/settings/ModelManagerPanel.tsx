import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { ConfiguredModel } from "@/api/types";
import { api } from "@/api/client";
import { CapabilityActiveMatrix } from "@/components/settings/CapabilityActiveMatrix";
import { ConfiguredModelsList } from "@/components/settings/ConfiguredModelsList";
import { ModelCatalogBrowser } from "@/components/settings/ModelCatalogBrowser";
import { ModelEditorDrawer } from "@/components/settings/ModelEditorDrawer";
import { ModelSettingsPanel } from "@/components/settings/ModelSettingsPanel";
import { RoutingAgentsEditor } from "@/components/settings/RoutingAgentsEditor";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

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

export function ModelManagerPanel() {
  const t = useT();
  const qc = useQueryClient();
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
    if (fromRegistry.length > 0) return fromRegistry;
    return maskToConfigured(llm.data?.registry?.items ?? []);
  }, [registryQuery.data?.items, llm.data?.registry?.items]);

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

  if (registryQuery.isLoading || catalog.isLoading) {
    return (
      <SectionCard title={t("settings.model.managerTitle")}>
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      </SectionCard>
    );
  }

  return (
    <>
      <SectionCard title={t("settings.model.managerTitle")}>
        <p className="text-sm text-secondary m-0 mb-4">{t("settings.model.managerHint")}</p>
        <button
          type="button"
          className="dw-btn-primary mb-4"
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
          {t("settings.model.addCustom")}
        </button>
      </SectionCard>

      <CapabilityActiveMatrix
        registry={registryQuery.data}
        items={items}
        enabling={enableCap.isPending}
        onEnable={(id, cap) => enableCap.mutate({ id, cap })}
      />

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

      <ModelSettingsPanel />

      <RoutingAgentsEditor />

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
