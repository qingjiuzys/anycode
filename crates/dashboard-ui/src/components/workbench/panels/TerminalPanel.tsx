import { useEffect, useRef, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { useTerminalAttachment } from "../hooks/useWorkbenchTerminal";

type Tab = { sessionId: string; index: number };

type Props = {
  projectId: string;
  conversationSessionId: string | null;
  active: boolean;
};

/** A single terminal tab — attaches to one live backend session. */
function TerminalTab({ sessionId, active }: { sessionId: string; active: boolean }) {
  const { containerRef } = useTerminalAttachment(sessionId, active);
  return (
    <div
      ref={containerRef}
      className="h-full min-h-[200px] p-1 bg-[#0d1117] overflow-hidden"
    />
  );
}

/**
 * Terminal session group: each workbench conversation owns one group of
 * persistent terminal tabs. Entering the panel does **not** create a new
 * session — it attaches to existing ones (creating one only if none exist).
 * Users click "+" to add another tab.
 */
export function TerminalPanel({ projectId, conversationSessionId, active }: Props) {
  const t = useT();
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTab, setActiveTab] = useState<string | null>(null);
  const [booting, setBooting] = useState(false);
  const bootedRef = useRef(false);
  const nextIndex = useRef(1);

  // Entering the panel: attach to existing sessions, don't force a new one.
  useEffect(() => {
    if (!active || !projectId || !conversationSessionId || bootedRef.current) return;
    bootedRef.current = true;
    let cancelled = false;
    setBooting(true);

    void api
      .listTerminalSessions(projectId, conversationSessionId)
      .then(async (data) => {
        if (cancelled) return;
        if (data.sessions.length > 0) {
          const init = data.sessions.map((s, i) => ({
            sessionId: s.session_id,
            index: i + 1,
          }));
          nextIndex.current = init.length + 1;
          setTabs(init);
          setActiveTab(init[0].sessionId);
        } else {
          const created = await api.createTerminalSession(projectId, conversationSessionId);
          if (cancelled) {
            void api.deleteTerminalSession(created.session.session_id);
            return;
          }
          nextIndex.current = 2;
          setTabs([{ sessionId: created.session.session_id, index: 1 }]);
          setActiveTab(created.session.session_id);
        }
      })
      .catch(() => {
        /* backend unavailable — leave panel empty */
      })
      .finally(() => setBooting(false));

    return () => {
      cancelled = true;
    };
  }, [active, projectId, conversationSessionId]);

  const addTab = () => {
    if (!projectId || !conversationSessionId) return;
    void api.createTerminalSession(projectId, conversationSessionId).then((data) => {
      const tab = { sessionId: data.session.session_id, index: nextIndex.current++ };
      setTabs((prev) => [...prev, tab]);
      setActiveTab(tab.sessionId);
    });
  };

  const closeTab = (sessionId: string) => {
    void api.deleteTerminalSession(sessionId);
    setTabs((prev) => {
      const next = prev.filter((x) => x.sessionId !== sessionId);
      if (activeTab === sessionId) {
        setActiveTab(next[next.length - 1]?.sessionId ?? null);
      }
      return next;
    });
  };

  return (
    <div className="flex flex-col h-full min-h-0">
      <div className="flex items-center gap-1 px-1 py-1 border-b border-outline-variant/60 bg-surface-container-low shrink-0 overflow-x-auto">
        {tabs.map((tab) => {
          const isActive = activeTab === tab.sessionId;
          return (
            <span
              key={tab.sessionId}
              role="tab"
              aria-selected={isActive}
              className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs whitespace-nowrap border cursor-pointer select-none ${
                isActive
                  ? "bg-surface-container-high text-on-surface border-outline-variant"
                  : "text-secondary border-transparent hover:bg-surface-container-high/60"
              }`}
              onClick={() => setActiveTab(tab.sessionId)}
            >
              {t("workbench.tabTerminal")} {tab.index}
              <button
                type="button"
                className="dw-btn-ghost p-0 text-[10px] leading-none"
                aria-label={t("workbench.terminalNewTab")}
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(tab.sessionId);
                }}
              >
                <Icon name="close" size={12} />
              </button>
            </span>
          );
        })}
        <button
          type="button"
          className="dw-btn-ghost p-0.5 text-secondary"
          title={t("workbench.terminalNewTab")}
          aria-label={t("workbench.terminalNewTab")}
          onClick={addTab}
        >
          <Icon name="add" size={16} />
        </button>
        {booting && <span className="text-[10px] text-secondary px-1">{t("common.loading")}</span>}
      </div>
      <div className="flex-1 min-h-[12rem] min-w-0">
        {activeTab ? (
          tabs.map((tab) =>
            tab.sessionId === activeTab ? (
              <TerminalTab key={tab.sessionId} sessionId={tab.sessionId} active={active} />
            ) : null,
          )
        ) : (
          <p className="text-xs text-secondary text-center py-8 m-0">
            {t("workbench.terminalEmpty")}
          </p>
        )}
      </div>
    </div>
  );
}