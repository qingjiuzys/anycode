import { useEffect, useId, useRef, useState } from "react";
import { useI18n } from "../i18n/context";

const LOCALES = [
  { id: "zh" as const, labelKey: "common.zh" },
  { id: "en" as const, labelKey: "common.en" },
];

export function LocaleSwitcher({ variant = "default" }: { variant?: "default" | "header" }) {
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
    <div
      className={`locale-dropdown${variant === "header" ? " locale-dropdown--header" : ""}${open ? " is-open" : ""}`}
      ref={rootRef}
    >
      <button
        type="button"
        className={`locale-dropdown__trigger${variant === "header" ? " locale-dropdown__trigger--icon" : ""}`}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
        aria-label={t("common.language")}
        title={t(current.labelKey)}
        onClick={() => setOpen((v) => !v)}
      >
        <svg className="locale-dropdown__globe" width="20" height="20" viewBox="0 0 24 24" aria-hidden>
          <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="1.75" />
          <path
            d="M3 12h18M12 3c2.5 2.8 3.8 5.8 3.8 9s-1.3 6.2-3.8 9M12 3c-2.5 2.8-3.8 5.8-3.8 9s1.3 6.2 3.8 9"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.75"
          />
        </svg>
        {variant === "header" ? null : (
          <>
            <span className="locale-dropdown__label">{t(current.labelKey)}</span>
            <svg className="locale-dropdown__chev" width="12" height="12" viewBox="0 0 12 12" aria-hidden>
              <path d="M2.5 4.5 6 8 9.5 4.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
            </svg>
          </>
        )}
      </button>
      {open && (
        <ul className="locale-dropdown__menu" role="listbox" id={listId} aria-label={t("common.language")}>
          {LOCALES.map((item) => (
            <li key={item.id} role="presentation">
              <button
                type="button"
                role="option"
                aria-selected={locale === item.id}
                className={`locale-dropdown__option${locale === item.id ? " is-active" : ""}`}
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
