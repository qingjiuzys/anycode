import type { ConfiguredModel, ModelsRegistryView } from "@/api/types";

export type ComposerModelOption = {
  id: string;
  label: string;
  subtitle: string;
};

export const COMPOSER_MODEL_STORAGE_KEY = "anycode-composer-model";
export const COMPOSER_AUTO_STORAGE_KEY = "anycode-composer-auto";

export function modelLabel(item: ConfiguredModel): string {
  return item.display_name?.trim() || `${item.provider}/${item.model}`;
}

export function modelSubtitle(item: ConfiguredModel): string {
  if (item.display_name?.trim()) {
    return `${item.provider}/${item.model}`;
  }
  return item.provider;
}

export function listChatModels(items: ConfiguredModel[]): ComposerModelOption[] {
  return items
    .filter((m) => m.enabled && m.capabilities.includes("chat"))
    .map((item) => ({
      id: item.id,
      label: modelLabel(item),
      subtitle: modelSubtitle(item),
    }))
    .sort((a, b) => a.label.localeCompare(b.label, undefined, { sensitivity: "base" }));
}

/** Registry item id for global provider/model, if present in chat-capable items. */
export function findGlobalDefaultChatId(registry?: ModelsRegistryView | null): string | null {
  if (!registry) return null;
  const provider = registry.global?.provider?.trim();
  const model = registry.global?.model?.trim();
  if (!provider || !model) return registry.active?.chat ?? null;
  const match = registry.items.find(
    (item) =>
      item.enabled &&
      item.capabilities.includes("chat") &&
      item.provider === provider &&
      item.model === model,
  );
  return match?.id ?? registry.active?.chat ?? null;
}

export function readStoredAuto(fallback: boolean): boolean {
  try {
    const v = localStorage.getItem(COMPOSER_AUTO_STORAGE_KEY);
    if (v === "1") return true;
    if (v === "0") return false;
  } catch {
    /* ignore */
  }
  return fallback;
}

export function writeStoredAuto(auto: boolean): void {
  try {
    localStorage.setItem(COMPOSER_AUTO_STORAGE_KEY, auto ? "1" : "0");
  } catch {
    /* ignore */
  }
}

export function writeStoredModelId(id: string): void {
  try {
    localStorage.setItem(COMPOSER_MODEL_STORAGE_KEY, id);
  } catch {
    /* ignore */
  }
}

/** Auto = active chat matches global default (routing picks per agent/mode). */
export function inferAutoFromRegistry(registry?: ModelsRegistryView | null): boolean {
  if (!registry) return true;
  const activeChat = registry.active?.chat;
  if (!activeChat) return true;
  const globalId = findGlobalDefaultChatId(registry);
  if (!globalId) return false;
  return activeChat === globalId;
}
