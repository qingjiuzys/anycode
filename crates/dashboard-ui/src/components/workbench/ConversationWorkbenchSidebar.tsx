import { useCallback, useEffect, useRef } from "react";
import type { TranscriptBlock } from "@/api/types";
import type { WorkbenchTab } from "@/api/types/workbench";
import { useT } from "@/i18n/context";
import { WorkbenchActivityRail } from "./WorkbenchActivityRail";
import { WorkbenchPanel } from "./WorkbenchPanel";
import { useWorkbenchSidebarState } from "./hooks/useWorkbenchSidebarState";
import { FilesPanel } from "./panels/FilesPanel";
import { BrowserPanel } from "./panels/BrowserPanel";
import { TerminalPanel } from "./panels/TerminalPanel";
import { ArtifactsPanel } from "./panels/ArtifactsPanel";
import { TracePanel } from "./panels/TracePanel";

function isBrowserToolBlock(block: TranscriptBlock): boolean {
  const metaName =
    typeof block.meta?.name === "string" ? block.meta.name.toLowerCase() : "";
  const haystack = `${block.title} ${block.body} ${metaName}`.toLowerCase();
  return haystack.includes("browser");
}

type Props = {
  projectId: string | null | undefined;
  sessionId: string | null;
  className?: string;
  live?: boolean;
  isRunning?: boolean;
  liveBlocks?: TranscriptBlock[];
  liveEvents?: import("@/lib/liveTranscript").ChatStreamEvent[];
  selectedTool?: TranscriptBlock | null;
  onSelectTool?: (tool: TranscriptBlock | null) => void;
  /** Parent dock controls visibility; always show expanded panel when mounted. */
  forceExpanded?: boolean;
  /** Collapse/close the pushed workbench dock. */
  onRequestClose?: () => void;
};

export function ConversationWorkbenchSidebar({
  projectId,
  sessionId,
  className = "",
  live,
  isRunning,
  liveBlocks,
  liveEvents = [],
  selectedTool,
  onSelectTool,
  forceExpanded,
  onRequestClose,
}: Props) {
  const t = useT();
  const { expanded, activeTab, panelWidth, selectTab, setExpanded, setPanelWidth, openTab } =
    useWorkbenchSidebarState();
  const resizeRef = useRef<{ startX: number; startW: number } | null>(null);

  const showPanel = forceExpanded || expanded;
  const disabled = !sessionId;
  const needsProject = activeTab === "files" || activeTab === "browser" || activeTab === "terminal";
  const projectReady = Boolean(projectId);

  useEffect(() => {
    if (selectedTool && sessionId) {
      openTab("trace");
    }
  }, [selectedTool?.id, sessionId, openTab]);

  // Prefer the live browser pane while agent drives Browser* tools (unless user
  // has drilled into a tool step → Trace, or already chose terminal/files).
  useEffect(() => {
    if (!sessionId || !showPanel || selectedTool) return;
    const hasBrowser = (liveBlocks ?? []).some(isBrowserToolBlock);
    if (!hasBrowser) return;
    if (activeTab === "browser" || activeTab === "terminal" || activeTab === "files") return;
    openTab("browser");
  }, [sessionId, showPanel, selectedTool, liveBlocks, activeTab, openTab]);

  const onResizeStart = useCallback(
    (e: React.PointerEvent) => {
      e.preventDefault();
      resizeRef.current = { startX: e.clientX, startW: panelWidth };
      const onMove = (ev: PointerEvent) => {
        if (!resizeRef.current) return;
        const delta = resizeRef.current.startX - ev.clientX;
        setPanelWidth(resizeRef.current.startW + delta);
      };
      const onUp = () => {
        resizeRef.current = null;
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
      };
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
    },
    [panelWidth, setPanelWidth],
  );

  const renderPanelContent = () => {
    if (disabled) {
      return (
        <p className="text-sm text-secondary px-4 py-6 m-0 text-center">
          {t("conversations.selectSession")}
        </p>
      );
    }
    if (needsProject && !projectReady) {
      return (
        <p className="text-sm text-secondary px-4 py-6 m-0 text-center">
          {t("workbench.noProject")}
        </p>
      );
    }
    switch (activeTab) {
      case "files":
        return <FilesPanel projectId={projectId!} />;
      case "browser":
        return (
          <BrowserPanel
            projectId={projectId!}
            conversationSessionId={sessionId}
            active={showPanel}
          />
        );
      case "terminal":
        return <TerminalPanel projectId={projectId!} active={showPanel} />;
      case "artifacts":
        return (
          <ArtifactsPanel sessionId={sessionId!} live={live} isRunning={isRunning} />
        );
      case "trace":
        return (
          <TracePanel
            sessionId={sessionId!}
            live={live}
            isRunning={isRunning}
            liveEvents={liveEvents}
            selectedTool={selectedTool}
            onSelectTool={onSelectTool}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className={`flex shrink-0 min-h-0 h-full ${className}`}>
      {showPanel && (
        <WorkbenchPanel
          activeTab={activeTab}
          width={panelWidth}
          onResizeStart={onResizeStart}
          onCollapse={() => {
            setExpanded(false);
            onRequestClose?.();
          }}
        >
          {renderPanelContent()}
        </WorkbenchPanel>
      )}
      <WorkbenchActivityRail
        activeTab={activeTab}
        onSelectTab={selectTab}
        disabled={disabled}
      />
    </div>
  );
}

export type { WorkbenchTab };
