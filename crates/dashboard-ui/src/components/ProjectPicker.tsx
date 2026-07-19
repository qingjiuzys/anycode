import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type ProjectOption = { id: string; name: string };

type Props = {
  value: string;
  onChange: (projectId: string) => void;
  options: ProjectOption[];
  disabled?: boolean;
  /** Opens the “select directory / register project” flow. */
  onSelectDirectory?: () => void;
};

export function ProjectPicker({
  value,
  onChange,
  options,
  disabled = false,
  onSelectDirectory,
}: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [menuPos, setMenuPos] = useState<{ left: number; top: number; width: number } | null>(
    null,
  );
  const selected = options.find((option) => option.id === value);

  useEffect(() => {
    if (!open) return;
    const update = () => {
      const el = triggerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const width = Math.max(rect.width, 220);
      const left = Math.min(rect.left, window.innerWidth - width - 12);
      // Open upward above the composer toolbar trigger
      const estimatedHeight = Math.min(280, 48 + options.length * 40 + (onSelectDirectory ? 52 : 0));
      const top = Math.max(12, rect.top - estimatedHeight - 8);
      setMenuPos({ left: Math.max(12, left), top, width });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open, options.length, onSelectDirectory]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (event: MouseEvent) => {
      const target = event.target as Node;
      if (menuRef.current?.contains(target) || triggerRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const triggerLabel =
    selected?.name ??
    (options.length === 0 ? t("home.hero.selectDirectory") : t("home.hero.projectLabel"));

  return (
    <div className="dw-project-picker">
      <button
        ref={triggerRef}
        type="button"
        className="dw-project-picker__trigger"
        disabled={disabled}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={t("home.hero.projectLabel")}
        title={selected?.name}
        onClick={() => setOpen((prev) => !prev)}
      >
        <span className="dw-project-picker__icon" aria-hidden>
          <Icon name="folder_open" size={16} />
        </span>
        <span className="dw-project-picker__label truncate min-w-0">{triggerLabel}</span>
        <Icon
          name="expand_more"
          size={16}
          className={`dw-project-picker__chevron shrink-0 text-secondary${open ? " dw-project-picker__chevron--open" : ""}`}
        />
      </button>

      {open &&
        menuPos &&
        createPortal(
          <div
            ref={menuRef}
            className="dw-project-picker__menu"
            style={{ left: menuPos.left, top: menuPos.top, width: menuPos.width }}
            role="listbox"
            aria-label={t("home.hero.currentDirectory")}
          >
            <div className="dw-project-picker__section-label">{t("home.hero.currentDirectory")}</div>
            {options.length === 0 ? (
              <p className="dw-project-picker__empty m-0">{t("home.hero.noProject")}</p>
            ) : (
              <div className="dw-project-picker__list">
                {options.map((option) => {
                  const active = option.id === value;
                  return (
                    <button
                      key={option.id}
                      type="button"
                      role="option"
                      aria-selected={active}
                      title={option.name}
                      className={`dw-project-picker__item${active ? " dw-project-picker__item--active" : ""}`}
                      onClick={() => {
                        onChange(option.id);
                        setOpen(false);
                      }}
                    >
                      <span className="dw-project-picker__item-icon" aria-hidden>
                        <Icon name="folder" size={16} />
                      </span>
                      <span className="truncate min-w-0 flex-1">{option.name}</span>
                      {active ? (
                        <Icon name="check" size={16} className="dw-project-picker__check shrink-0" />
                      ) : null}
                    </button>
                  );
                })}
              </div>
            )}
            {onSelectDirectory ? (
              <>
                <div className="dw-project-picker__divider" role="separator" />
                <button
                  type="button"
                  className="dw-project-picker__item dw-project-picker__item--action"
                  onClick={() => {
                    setOpen(false);
                    onSelectDirectory();
                  }}
                >
                  <span className="dw-project-picker__item-icon" aria-hidden>
                    <Icon name="add" size={16} />
                  </span>
                  <span className="truncate min-w-0 flex-1">{t("home.hero.selectDirectory")}</span>
                </button>
              </>
            ) : null}
          </div>,
          document.body,
        )}
    </div>
  );
}
