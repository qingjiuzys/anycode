import type { ConfiguredModel, ModelsRegistryView } from "@/api/types";

export type ComposerModelOption = {
  id: string;
  label: string;
  subtitle: string;
  isCloud?: boolean;
  cloudModel?: string;
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
  const seen = new Set<string>();
  const options = items
    .filter((m) => m.enabled && m.capabilities.includes("chat"))
    .filter((m) => {
      const key = `${m.provider}/${m.model}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .map((item) => ({
      id: item.id,
      label: modelLabel(item),
      subtitle: modelSubtitle(item),
      isCloud: item.source === "cloud",
      cloudModel: item.model,
    }));

  return options.sort((a, b) => {
    const rank = (o: ComposerModelOption) => {
      if (o.isCloud && o.cloudModel === "auto") return 0;
      if (o.isCloud) return 1;
      return 2;
    };
    const dr = rank(a) - rank(b);
    if (dr !== 0) return dr;
    return a.label.localeCompare(b.label, undefined, { sensitivity: "base" });
  });
}

export function readStoredModelId(): string | null {
  try {
    const v = localStorage.getItem(COMPOSER_MODEL_STORAGE_KEY);
    return v?.trim() || null;
  } catch {
    return null;
  }
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
