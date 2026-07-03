import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { useEmbeddedControlCenter } from "@/context/EmbeddedControlCenterContext";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useT } from "@/i18n/context";
import {
  findGlobalDefaultChatId,
  inferAutoFromRegistry,
  listChatModels,
  modelLabel,
  readStoredAuto,
  writeStoredAuto,
  writeStoredModelId,
  type ComposerModelOption,
} from "@/lib/composerModels";

type Props = {
  disabled?: boolean;
  compact?: boolean;
};

export function ModelPicker({ disabled = false, compact = false }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const embedded = useEmbeddedControlCenter();
  const { openControlCenter } = useControlCenter();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [auto, setAuto] = useState(() => readStoredAuto(true));
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const syncedRef = useRef(false);

  const registry = useQuery({
    queryKey: ["models-registry"],
    queryFn: () => api.getModelsRegistry(),
    staleTime: 60_000,
  });

  const enableModel = useMutation({
    mutationFn: ({ id }: { id: string }) => api.enableModel(id, ["chat"]),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["models-registry"] });
      void queryClient.invalidateQueries({ queryKey: ["llm-config"] });
    },
  });

  const registryItems = registry.data?.items ?? [];
  const chatOptions = useMemo(() => listChatModels(registryItems), [registryItems]);

  const selectedOption = useMemo((): ComposerModelOption | null => {
    if (!selectedId) return null;
    return chatOptions.find((m) => m.id === selectedId) ?? null;
  }, [chatOptions, selectedId]);

  const globalDefaultId = useMemo(
    () => findGlobalDefaultChatId(registry.data),
    [registry.data],
  );

  const globalModelLabel = useMemo(() => {
    const gp = registry.data?.global?.provider;
    const gm = registry.data?.global?.model;
    if (gp && gm) return `${gp}/${gm}`;
    if (globalDefaultId) {
      const item = registryItems.find((m) => m.id === globalDefaultId);
      if (item) return modelLabel(item);
    }
    return null;
  }, [registry.data, globalDefaultId, registryItems]);

  useEffect(() => {
    if (!registry.data || syncedRef.current) return;
    syncedRef.current = true;
    const storedAuto = readStoredAuto(inferAutoFromRegistry(registry.data));
    const activeChat = registry.data.active?.chat ?? null;
    setAuto(storedAuto);
    if (!storedAuto && activeChat) {
      setSelectedId(activeChat);
      return;
    }
    if (globalDefaultId) {
      setSelectedId(globalDefaultId);
    } else if (activeChat) {
      setSelectedId(activeChat);
    } else if (chatOptions[0]) {
      setSelectedId(chatOptions[0].id);
    }
  }, [registry.data, globalDefaultId, chatOptions]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return chatOptions;
    return chatOptions.filter(
      (m) =>
        m.label.toLowerCase().includes(q) ||
        m.subtitle.toLowerCase().includes(q) ||
        m.id.toLowerCase().includes(q),
    );
  }, [query, chatOptions]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  function applyAutoMode(next: boolean) {
    setAuto(next);
    writeStoredAuto(next);
    if (next) {
      const defaultId = findGlobalDefaultChatId(registry.data);
      if (defaultId) {
        setSelectedId(defaultId);
        enableModel.mutate({ id: defaultId });
      }
      return;
    }
    const activeChat = registry.data?.active?.chat;
    if (activeChat) {
      setSelectedId(activeChat);
      writeStoredModelId(activeChat);
    }
  }

  function pickModel(option: ComposerModelOption) {
    setSelectedId(option.id);
    writeStoredModelId(option.id);
    setAuto(false);
    writeStoredAuto(false);
    enableModel.mutate({ id: option.id });
    setOpen(false);
    setQuery("");
  }

  function openModelSettings() {
    setOpen(false);
    if (embedded) {
      openControlCenter("/settings?section=model");
    }
  }

  const triggerLabel = auto
    ? t("modelPicker.auto")
    : selectedOption?.label ?? t("modelPicker.selectModel");

  const triggerTitle =
    auto && globalModelLabel
      ? t("modelPicker.autoUses").replace("{model}", globalModelLabel)
      : selectedOption
        ? `${selectedOption.label}${selectedOption.subtitle !== selectedOption.label ? ` (${selectedOption.subtitle})` : ""}`
        : undefined;

  return (
    <div ref={rootRef} className={`dw-model-picker relative ${compact ? "dw-model-picker--compact" : ""}`}>
      <button
        type="button"
        className="dw-model-picker__trigger"
        disabled={disabled}
        aria-expanded={open}
        aria-haspopup="listbox"
        title={triggerTitle}
        onClick={() => setOpen((v) => !v)}
      >
        <span className="truncate min-w-0">{triggerLabel}</span>
        <Icon name="expand_more" size={14} className="shrink-0 text-secondary" />
      </button>

      {open && (
        <div className="dw-model-picker__menu glass-panel" role="listbox">
          <div className="dw-model-picker__search-wrap">
            <Icon name="search" size={16} className="text-secondary shrink-0" />
            <input
              type="search"
              className="dw-model-picker__search"
              placeholder={t("modelPicker.search")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              autoFocus
            />
          </div>

          <div className="dw-model-picker__toggle-row">
            <div className="flex flex-col gap-0.5 min-w-0">
              <span>{t("modelPicker.auto")}</span>
              {globalModelLabel && (
                <span className="text-[11px] text-secondary truncate">
                  {t("modelPicker.autoUses").replace("{model}", globalModelLabel)}
                </span>
              )}
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={auto}
              className={`dw-model-picker__switch${auto ? " dw-model-picker__switch--on" : ""}`}
              onClick={() => applyAutoMode(!auto)}
            />
          </div>

          <div className="dw-model-picker__list">
            {filtered.length === 0 ? (
              <p className="text-xs text-secondary px-3 py-4 m-0 text-center">
                {t("modelPicker.noModels")}
              </p>
            ) : (
              filtered.map((option) => {
                const active = !auto && selectedId === option.id;
                return (
                  <button
                    key={option.id}
                    type="button"
                    role="option"
                    aria-selected={active}
                    className={`dw-model-picker__item${active ? " dw-model-picker__item--active" : ""}`}
                    onClick={() => pickModel(option)}
                  >
                    <div className="dw-model-picker__item-head">
                      <span className="dw-model-picker__item-label">{option.label}</span>
                      {active && <Icon name="check" size={16} className="text-primary shrink-0" />}
                    </div>
                    {option.subtitle !== option.label && (
                      <span className="dw-model-picker__item-tier">{option.subtitle}</span>
                    )}
                  </button>
                );
              })
            )}
          </div>

          <div className="dw-model-picker__footer">
            {embedded ? (
              <button type="button" className="dw-model-picker__add" onClick={openModelSettings}>
                {t("modelPicker.addModels")}
              </button>
            ) : (
              <ControlCenterLink
                to="/settings"
                search={{ section: "model" }}
                className="dw-model-picker__add"
                onClick={() => setOpen(false)}
              >
                {t("modelPicker.addModels")}
              </ControlCenterLink>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
