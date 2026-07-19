import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { useEmbeddedControlCenter } from "@/context/EmbeddedControlCenterContext";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useT } from "@/i18n/context";
import {
  listChatModels,
  readStoredModelId,
  writeStoredModelId,
  type ComposerModelOption,
} from "@/lib/composerModels";

function formatModelLabel(
  option: ComposerModelOption,
  t: (key: string) => string,
): string {
  if (option.isCloud && option.cloudModel === "auto") {
    return t("modelPicker.cloudAuto");
  }
  if (option.isCloud) {
    return t("modelPicker.cloudNamed").replace("{name}", option.label);
  }
  return option.label;
}

const MENU_WIDTH = 320;

function useAnchoredMenuStyle(open: boolean, anchorRef: React.RefObject<HTMLElement | null>) {
  const [style, setStyle] = useState<CSSProperties>({});

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) return;
    const update = () => {
      const rect = anchorRef.current!.getBoundingClientRect();
      const width = Math.min(MENU_WIDTH, window.innerWidth - 16);
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
      setStyle({
        position: "fixed",
        left,
        bottom: window.innerHeight - rect.top + 8,
        width,
        zIndex: 300,
      });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open, anchorRef]);

  return style;
}

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
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const menuStyle = useAnchoredMenuStyle(open, triggerRef);

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

  useEffect(() => {
    if (!registry.data) return;
    const activeChat = registry.data.active?.chat ?? null;
    const stored = readStoredModelId();
    const storedValid = stored && chatOptions.some((m) => m.id === stored) ? stored : null;
    const next = activeChat ?? storedValid ?? chatOptions[0]?.id ?? null;
    setSelectedId(next);
  }, [registry.data, chatOptions]);

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
      const target = e.target as Node;
      if (rootRef.current?.contains(target)) return;
      if (menuRef.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  function pickModel(option: ComposerModelOption) {
    setSelectedId(option.id);
    writeStoredModelId(option.id);
    enableModel.mutate(
      { id: option.id },
      {
        onSuccess: () => {
          void queryClient.invalidateQueries({ queryKey: ["models-registry"] });
          void queryClient.invalidateQueries({ queryKey: ["llm-config"] });
        },
      },
    );
    setOpen(false);
    setQuery("");
  }

  function openModelSettings() {
    setOpen(false);
    if (embedded) {
      openControlCenter("/settings?section=model");
    }
  }

  const triggerLabel = selectedOption
    ? formatModelLabel(selectedOption, t)
    : t("modelPicker.selectModel");
  const triggerTitle = selectedOption
    ? selectedOption.subtitle !== selectedOption.label
      ? `${selectedOption.label} · ${selectedOption.subtitle}`
      : selectedOption.label
    : undefined;

  return (
    <div ref={rootRef} className={`dw-model-picker relative ${compact ? "dw-model-picker--compact" : ""}`}>
      <button
        ref={triggerRef}
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

      {open &&
        createPortal(
          <div
            ref={menuRef}
            className="dw-model-picker__menu dw-model-picker__menu--anchored glass-panel"
            style={menuStyle}
            role="listbox"
            aria-label={t("modelPicker.selectModel")}
          >
            {chatOptions.length > 6 && (
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
            )}

            <div className="dw-model-picker__list">
              {filtered.length === 0 ? (
                <p className="text-xs text-secondary px-3 py-4 m-0 text-center">
                  {t("modelPicker.noModels")}
                </p>
              ) : (
                filtered.map((option) => {
                  const active = selectedId === option.id;
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
                        <span className="dw-model-picker__item-label">{formatModelLabel(option, t)}</span>
                        {option.isCloud && (
                          <span className="dw-model-picker__cloud-badge">{t("modelPicker.cloudBadge")}</span>
                        )}
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
          </div>,
          document.body,
        )}
    </div>
  );
}
