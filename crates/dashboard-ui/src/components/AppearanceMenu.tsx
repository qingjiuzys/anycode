import { useEffect, useRef, useState } from "react";
import { Link } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { SkinPickerCompact } from "@/components/SkinPicker";
import { ThemeModeSwitch } from "@/components/ThemeModeSwitch";
import { useT } from "@/i18n/context";

export function AppearanceMenu() {
  const t = useT();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        className={`dw-nav-link w-full ${open ? "active" : ""}`}
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        aria-haspopup="dialog"
      >
        <Icon name="palette" size={18} />
        <span className="flex-1 min-w-0 truncate text-left">{t("layout.appearance")}</span>
        <Icon name={open ? "expand_less" : "expand_more"} size={16} className="text-secondary" />
      </button>

      {open && (
        <div className="appearance-menu-popover" role="dialog" aria-label={t("layout.appearance")}>
          <div className="appearance-menu-popover__section">
            <p className="appearance-menu-popover__label">{t("settings.appearance.themeLabel")}</p>
            <ThemeModeSwitch />
          </div>
          <div className="appearance-menu-popover__section">
            <p className="appearance-menu-popover__label">{t("settings.appearance.skinLabel")}</p>
            <SkinPickerCompact />
          </div>
          <Link
            to="/settings"
            search={{ section: "prefs" }}
            className="appearance-menu-popover__link"
            onClick={() => setOpen(false)}
          >
            <Icon name="settings" size={16} />
            {t("settings.appearance.openSettings")}
          </Link>
        </div>
      )}
    </div>
  );
}
