import { useEffect, useState } from "react";
import type { CatalogProviderRow, ConfiguredModel } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const ALL_CAPS = ["chat", "vision", "embedding", "stt", "tts", "image", "video"];

type Props = {
  open: boolean;
  draft: ConfiguredModel | null;
  providers: CatalogProviderRow[];
  onClose: () => void;
  onSave: (item: ConfiguredModel) => void;
  onTest: (item: ConfiguredModel) => void;
  testing?: boolean;
};

export function ModelEditorDrawer({
  open,
  draft,
  providers,
  onClose,
  onSave,
  onTest,
  testing,
}: Props) {
  const t = useT();
  const [form, setForm] = useState<ConfiguredModel | null>(null);

  useEffect(() => {
    setForm(draft);
  }, [draft]);

  if (!open || !form) return null;

  const toggleCap = (cap: string) => {
    setForm((prev) => {
      if (!prev) return prev;
      const has = prev.capabilities.includes(cap);
      return {
        ...prev,
        capabilities: has
          ? prev.capabilities.filter((c) => c !== cap)
          : [...prev.capabilities, cap],
      };
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-scrim/40 p-4">
      <SectionCard
        title={draft?.source === "catalog" ? t("settings.model.addModel") : t("settings.model.editModel")}
        className="w-full max-w-2xl max-h-[90vh] overflow-y-auto shadow-lg"
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
          <label className="flex flex-col gap-1 text-sm sm:col-span-2">
            <span className="text-secondary font-medium">ID</span>
            <input
              className="dw-input font-code"
              value={form.id}
              onChange={(e) => setForm({ ...form, id: e.target.value })}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm sm:col-span-2">
            <span className="text-secondary font-medium">{t("settings.model.displayName")}</span>
            <input
              className="dw-input"
              value={form.display_name ?? ""}
              onChange={(e) => setForm({ ...form, display_name: e.target.value || undefined })}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("settings.model.provider")}</span>
            <select
              className="dw-input font-code"
              value={form.provider}
              onChange={(e) => setForm({ ...form, provider: e.target.value })}
            >
              {providers.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex flex-col gap-1 text-sm">
            <span className="text-secondary font-medium">{t("settings.model.model")}</span>
            <input
              className="dw-input font-code"
              value={form.model}
              onChange={(e) => setForm({ ...form, model: e.target.value })}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm sm:col-span-2">
            <span className="text-secondary font-medium">{t("settings.model.baseUrl")}</span>
            <input
              className="dw-input font-code"
              value={form.base_url ?? ""}
              onChange={(e) => setForm({ ...form, base_url: e.target.value || undefined })}
            />
          </label>
          <label className="flex flex-col gap-1 text-sm sm:col-span-2">
            <span className="text-secondary font-medium">{t("settings.model.apiKey")}</span>
            <input
              type="password"
              className="dw-input font-code"
              value={form.api_key ?? ""}
              onChange={(e) => setForm({ ...form, api_key: e.target.value || undefined })}
              placeholder={t("settings.model.apiKeyOptional")}
              autoComplete="off"
            />
          </label>
        </div>

        <div className="mb-4">
          <span className="text-sm text-secondary font-medium">{t("settings.model.capabilitiesTitle")}</span>
          <div className="flex flex-wrap gap-2 mt-2">
            {ALL_CAPS.map((cap) => (
              <label key={cap} className="inline-flex items-center gap-1 text-sm">
                <input
                  type="checkbox"
                  checked={form.capabilities.includes(cap)}
                  onChange={() => toggleCap(cap)}
                />
                {t(`settings.model.capabilities.${cap}` as "settings.model.capabilities.chat")}
              </label>
            ))}
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          <button type="button" className="dw-btn-primary" onClick={() => onSave(form)}>
            {t("common.save")}
          </button>
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={testing}
            onClick={() => onTest(form)}
          >
            {testing ? t("common.loading") : t("settings.model.testConnection")}
          </button>
          <button type="button" className="dw-btn-secondary" onClick={onClose}>
            {t("common.back")}
          </button>
        </div>
      </SectionCard>
    </div>
  );
}
