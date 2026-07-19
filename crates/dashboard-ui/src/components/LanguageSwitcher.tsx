import { useEffect, useId, useRef, useState } from "react";
import { Icon } from "@/components/Icon";
import { useI18n } from "@/i18n/context";

const LOCALES = [
  { id: "zh" as const, labelKey: "common.zh" },
  { id: "en" as const, labelKey: "common.en" },
];

export function LanguageSwitcher() {
  const { locale, setLocale, t } = useI18n();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const current = LOCALES.find((l) => l.id === locale) ?? LOCALES[0];

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div className={`dw-locale-switch${open ? " is-open" : ""}`} ref={rootRef}>
      <button
        type="button"
        className="dw-topbar-control dw-topbar-control--secondary dw-locale-switch__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        aria-label={t("common.language")}
        onClick={() => setOpen((v) => !v)}
      >
        <Icon name="language" size={16} className="dw-locale-switch__icon" />
        <span className="dw-locale-switch__label">{t(current.labelKey)}</span>
        <Icon name="expand_more" size={16} className="dw-locale-switch__chev" />
      </button>
      {open && (
        <ul className="dw-locale-switch__menu" role="listbox" id={listId}>
          {LOCALES.map((item) => (
            <li key={item.id} role="presentation">
              <button
                type="button"
                role="option"
                aria-selected={locale === item.id}
                className={`dw-locale-switch__option${locale === item.id ? " is-active" : ""}`}
                onClick={() => {
                  setLocale(item.id);
                  setOpen(false);
                }}
              >
                {t(item.labelKey)}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
