import { useEffect, useRef, useState } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type ProjectOption = { id: string; name: string };

type Props = {
  value: string;
  onChange: (projectId: string) => void;
  options: ProjectOption[];
  disabled?: boolean;
};

export function ProjectPicker({ value, onChange, options, disabled = false }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const selected = options.find((option) => option.id === value);

  useEffect(() => {
    if (!open) return;
    const onDoc = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  return (
    <div ref={rootRef} className="dw-project-picker">
      <button
        type="button"
        className="dw-project-picker__trigger"
        disabled={disabled || options.length === 0}
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-label={t("home.hero.projectLabel")}
        onClick={() => setOpen((prev) => !prev)}
      >
        <Icon name="folder" size={16} className="text-secondary shrink-0" />
        <span className="truncate min-w-0">
          {selected?.name ?? (options.length === 0 ? t("home.hero.noProject") : t("home.hero.projectLabel"))}
        </span>
        <Icon name="expand_more" size={14} className="text-secondary shrink-0" />
      </button>

      {open && options.length > 0 && (
        <div className="dw-project-picker__menu glass-panel" role="listbox">
          {options.map((option) => {
            const active = option.id === value;
            return (
              <button
                key={option.id}
                type="button"
                role="option"
                aria-selected={active}
                className={`dw-project-picker__item${active ? " dw-project-picker__item--active" : ""}`}
                onClick={() => {
                  onChange(option.id);
                  setOpen(false);
                }}
              >
                <span className="truncate">{option.name}</span>
                {active ? <Icon name="check" size={14} className="shrink-0 text-primary" /> : null}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
