import { useEffect, useRef, useState } from "react";
import { Icon } from "@/components/Icon";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { AutomationCreateDialog } from "@/components/AutomationCreatePanel";
import { useT } from "@/i18n/context";

/** Topbar "+ New" dropdown: new project / new scheduled cron job. */
export function TopbarNewMenu() {
  const t = useT();
  const [menuOpen, setMenuOpen] = useState(false);
  const [projectOpen, setProjectOpen] = useState(false);
  const [automationOpen, setAutomationOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, []);

  return (
    <div className="relative hidden sm:block" ref={ref}>
      <button
        type="button"
        className="dw-btn-primary"
        onClick={() => setMenuOpen((v) => !v)}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
      >
        <Icon name="add" size={16} />
        {t("layout.newMenu")}
        <Icon name="expand_more" size={16} />
      </button>
      {menuOpen && (
        <div
          className="absolute right-0 top-full mt-2 w-52 bg-surface-container-lowest border border-outline-variant rounded-lg shadow-lg z-50 py-1"
          role="menu"
        >
          <button
            type="button"
            role="menuitem"
            className="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-left hover:bg-surface-container text-on-surface border-0 bg-transparent cursor-pointer"
            onClick={() => {
              setMenuOpen(false);
              setProjectOpen(true);
            }}
          >
            <Icon name="folder" size={18} />
            {t("layout.newProject")}
          </button>
          <button
            type="button"
            role="menuitem"
            className="w-full flex items-center gap-2 px-4 py-2.5 text-sm text-left hover:bg-surface-container text-on-surface border-0 bg-transparent cursor-pointer"
            onClick={() => {
              setMenuOpen(false);
              setAutomationOpen(true);
            }}
          >
            <Icon name="settings_suggest" size={18} />
            {t("layout.newCronJob")}
          </button>
        </div>
      )}
      <NewProjectDialog open={projectOpen} onClose={() => setProjectOpen(false)} />
      <AutomationCreateDialog open={automationOpen} onClose={() => setAutomationOpen(false)} />
    </div>
  );
}
