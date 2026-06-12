import { useEffect, type ReactNode } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export type HomePanelSection = {
  id: string;
  title: string;
  content: ReactNode;
};

export function HomePanelOverlays({
  sections,
  activeId,
  onActiveChange,
}: {
  sections: HomePanelSection[];
  activeId: string | null;
  onActiveChange: (id: string | null) => void;
}) {
  const t = useT();
  const active = sections.find((section) => section.id === activeId);

  useEffect(() => {
    if (!activeId) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onActiveChange(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activeId, onActiveChange]);

  return (
    <>
      {active && (
        <>
          <button
            type="button"
            className="fixed inset-0 z-[90] border-0 bg-on-surface/20 cursor-default"
            style={{ top: "var(--dw-topbar-height, 3rem)" }}
            aria-label={t("home.panelClose")}
            onClick={() => onActiveChange(null)}
          />
          <div
            role="dialog"
            aria-modal
            aria-labelledby="home-panel-title"
            className={`fixed left-1/2 top-1/2 z-[95] -translate-x-1/2 -translate-y-1/2 max-h-[min(88vh,calc(100vh-3rem))] flex flex-col rounded-xl border border-outline-variant bg-surface-container-lowest shadow-xl overflow-hidden ${
              active.id === "analytics"
                ? "w-[min(52rem,calc(100vw-1.5rem))]"
                : "w-[min(44rem,calc(100vw-2rem))]"
            }`}
          >
            <div className="flex items-center justify-between gap-3 px-4 py-3 border-b border-outline-variant bg-surface-bright shrink-0">
              <h2 id="home-panel-title" className="text-sm font-semibold m-0 truncate">
                {active.title}
              </h2>
              <button
                type="button"
                className="dw-btn-ghost p-1 shrink-0"
                aria-label={t("home.panelClose")}
                onClick={() => onActiveChange(null)}
              >
                <Icon name="close" size={20} />
              </button>
            </div>
            <div
              className={`overflow-y-auto overscroll-y-contain flex-1 min-h-0 ${
                active.id === "analytics" ? "p-4 sm:p-5" : "p-4 space-y-4"
              }`}
            >
              {active.content}
            </div>
          </div>
        </>
      )}
    </>
  );
}

