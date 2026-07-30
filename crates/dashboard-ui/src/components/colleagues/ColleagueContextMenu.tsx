import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useT } from "@/i18n/context";

type MenuState = {
  peerId: string;
  x: number;
  y: number;
};

type Props = {
  onProjectHandoff: (peerId: string) => void;
  onSessionHandoff: (peerId: string) => void;
  /** Expose open-at-pointer so click and right-click can share the same menu. */
  onReady?: (open: (peerId: string, event: React.MouseEvent) => void) => void;
  children: React.ReactNode;
};

export function ColleagueContextMenu({
  onProjectHandoff,
  onSessionHandoff,
  onReady,
  children,
}: Props) {
  const t = useT();
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [submenuOpen, setSubmenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menu) return;
    const onPointerDown = (event: MouseEvent) => {
      if (menuRef.current?.contains(event.target as Node)) return;
      setMenu(null);
      setSubmenuOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenu(null);
        setSubmenuOpen(false);
      }
    };
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [menu]);

  const onContextMenu = useCallback((peerId: string, event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    setSubmenuOpen(false);
    setMenu({ peerId, x: event.clientX, y: event.clientY });
  }, []);

  useEffect(() => {
    onReady?.(onContextMenu);
  }, [onReady, onContextMenu]);

  return (
    <>
      {children}
      {menu &&
        createPortal(
          <div
            ref={menuRef}
            className="fixed z-[120] min-w-[11rem] rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg py-1"
            style={{ left: menu.x, top: menu.y }}
            role="menu"
          >
            <div className="relative">
              <button
                type="button"
                role="menuitem"
                className="w-full text-left px-3 py-2 text-sm border-0 bg-transparent hover:bg-surface-container-low cursor-pointer flex items-center justify-between gap-2"
                onMouseEnter={() => setSubmenuOpen(true)}
                onClick={() => setSubmenuOpen((v) => !v)}
              >
                {t("colleagues.handoff")}
                <span aria-hidden>▸</span>
              </button>
              {submenuOpen ? (
                <div className="absolute left-full top-0 ml-1 min-w-[10rem] rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg py-1">
                  <button
                    type="button"
                    role="menuitem"
                    className="w-full text-left px-3 py-2 text-sm border-0 bg-transparent hover:bg-surface-container-low cursor-pointer"
                    onClick={() => {
                      onProjectHandoff(menu.peerId);
                      setMenu(null);
                      setSubmenuOpen(false);
                    }}
                  >
                    {t("colleagues.projectHandoff")}
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    className="w-full text-left px-3 py-2 text-sm border-0 bg-transparent hover:bg-surface-container-low cursor-pointer"
                    onClick={() => {
                      onSessionHandoff(menu.peerId);
                      setMenu(null);
                      setSubmenuOpen(false);
                    }}
                  >
                    {t("colleagues.sessionHandoff")}
                  </button>
                </div>
              ) : null}
            </div>
          </div>,
          document.body,
        )}
    </>
  );
}
