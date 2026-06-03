import type { ConfiguredModel, ModelsRegistryView } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

const CAPABILITIES = ["chat", "vision", "embedding", "stt", "tts", "image", "video"] as const;

type Props = {
  registry?: ModelsRegistryView;
  items: ConfiguredModel[];
  onEnable: (modelId: string, capability: string) => void;
  enabling?: boolean;
};

function labelForItem(items: ConfiguredModel[], id: string) {
  const item = items.find((m) => m.id === id);
  if (!item) return id;
  return item.display_name ?? `${item.provider}/${item.model}`;
}

export function CapabilityActiveMatrix({ registry, items, onEnable, enabling }: Props) {
  const t = useT();
  const active = registry?.active ?? {};

  return (
    <SectionCard title={t("settings.model.activeMatrixTitle")}>
      <p className="text-xs text-secondary m-0 mb-3">{t("settings.model.activeMatrixHint")}</p>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
        {CAPABILITIES.map((cap) => {
          const activeId = active[cap];
          const activeItem = activeId ? items.find((m) => m.id === activeId) : undefined;
          return (
            <div
              key={cap}
              className="border border-outline-variant rounded-lg p-3 flex flex-col gap-2"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-medium uppercase tracking-wide text-secondary">
                  {t(`settings.model.capabilities.${cap}` as "settings.model.capabilities.chat")}
                </span>
                {activeId ? (
                  <StatusBadge status="ok" />
                ) : (
                  <StatusBadge status="pending" />
                )}
              </div>
              <p className="text-sm font-code m-0 truncate" title={activeId ? labelForItem(items, activeId) : undefined}>
                {activeId ? labelForItem(items, activeId) : t("settings.model.notConfigured")}
              </p>
              {activeItem && (
                <select
                  className="dw-input text-xs font-code"
                  value={activeId ?? ""}
                  disabled={enabling}
                  onChange={(e) => {
                    const next = e.target.value;
                    if (next) onEnable(next, cap);
                  }}
                >
                  <option value={activeId}>{t("settings.model.keepActive")}</option>
                  {items
                    .filter((m) => m.enabled && m.capabilities.includes(cap))
                    .filter((m) => m.id !== activeId)
                    .map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.display_name ?? `${m.provider}/${m.model}`}
                      </option>
                    ))}
                </select>
              )}
            </div>
          );
        })}
      </div>
    </SectionCard>
  );
}
