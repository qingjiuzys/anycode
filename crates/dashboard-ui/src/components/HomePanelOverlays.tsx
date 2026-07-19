import type { ReactNode } from "react";
import { Icon } from "@/components/Icon";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useT } from "@/i18n/context";

export type HomePanelSection = {
  id: string;
  title: string;
  content: ReactNode;
};

/** Above `.dw-control-center` (z-300) so overview pills work when embedded. */
const HOME_PANEL_Z_INDEX = 350;

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

  return (
    <ModalOverlay
      open={active != null}
      onClose={() => onActiveChange(null)}
      labelledBy="home-panel-title"
      zIndex={HOME_PANEL_Z_INDEX}
      className={`flex flex-col max-h-[min(92vh,calc(100vh-2rem))] rounded-xl border border-outline-variant bg-surface-container-lowest shadow-xl overflow-hidden ${
        active?.id === "analytics" || active?.id === "briefing"
          ? "w-[min(72rem,calc(100vw-1.5rem))]"
          : "w-[min(44rem,calc(100vw-2rem))]"
      }`}
    >
      {active && (
        <>
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
        </>
      )}
    </ModalOverlay>
  );
}
