import { useEffect, useRef, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { Icon } from "@/components/Icon";
import { NewProjectDialog } from "@/components/NewProjectDialog";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useT } from "@/i18n/context";
import { buildCreateViaChatPrompt } from "@/lib/automationChatPrompts";
import { setComposerSeed } from "@/lib/composerSeed";

/** Topbar "+ New" dropdown: new project / new scheduled cron job via chat. */
export function TopbarNewMenu() {
  const t = useT();
  const navigate = useNavigate();
  const { closeControlCenter } = useControlCenter();
  const [menuOpen, setMenuOpen] = useState(false);
  const [projectOpen, setProjectOpen] = useState(false);
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

  const openCronViaChat = () => {
    setMenuOpen(false);
    setComposerSeed(buildCreateViaChatPrompt(t));
    closeControlCenter();
    void navigate({ to: "/" });
  };

  return (
    <div className="relative hidden sm:block" ref={ref}>
      <button
        type="button"
        className="dw-topbar-control dw-topbar-control--primary"
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
            onClick={openCronViaChat}
          >
            <Icon name="settings_suggest" size={18} />
            {t("layout.newCronJob")}
          </button>
        </div>
      )}
      <NewProjectDialog open={projectOpen} onClose={() => setProjectOpen(false)} />
    </div>
  );
}
