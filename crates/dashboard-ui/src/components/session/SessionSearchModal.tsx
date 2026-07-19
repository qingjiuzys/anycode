import { useEffect, useMemo, useRef, useState } from "react";
import type { SessionWithProject } from "@/api/types";
import { Icon } from "@/components/Icon";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useT } from "@/i18n/context";
import { formatRelativeTime } from "@/utils/formatTime";

type Props = {
  open: boolean;
  onClose: () => void;
  sessions: SessionWithProject[];
  onSelect: (sessionId: string) => void;
};

export function SessionSearchModal({ open, onClose, sessions, onSelect }: Props) {
  const t = useT();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");

  useEffect(() => {
    if (!open) {
      setQuery("");
      return;
    }
    const id = window.requestAnimationFrame(() => inputRef.current?.focus());
    return () => window.cancelAnimationFrame(id);
  }, [open]);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sessions.slice(0, 40);
    return sessions
      .filter((s) => {
        const hay = `${s.title} ${s.id} ${s.project_name ?? ""}`.toLowerCase();
        return hay.includes(q);
      })
      .slice(0, 40);
  }, [query, sessions]);

  return (
    <ModalOverlay
      open={open}
      onClose={onClose}
      labelledBy="session-search-title"
      className="w-full max-w-lg"
      zIndex={320}
    >
      <div className="glass-modal rounded-2xl shadow-xl overflow-hidden flex flex-col max-h-[min(80dvh,560px)]">
        <div className="px-4 pt-4 pb-3 border-b border-outline-variant shrink-0">
          <div className="flex items-center justify-between gap-2 mb-3">
            <h2 id="session-search-title" className="text-base font-semibold m-0">
              {t("sidebar.search")}
            </h2>
            <button
              type="button"
              className="dw-btn-ghost p-1"
              onClick={onClose}
              aria-label={t("controlCenter.close")}
            >
              <Icon name="close" size={18} />
            </button>
          </div>
          <div className="relative">
            <Icon
              name="search"
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-outline pointer-events-none"
            />
            <input
              ref={inputRef}
              type="search"
              className="dw-input w-full pl-9 text-sm"
              placeholder={t("conversations.sessionSearch")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
        </div>
        <ul className="m-0 p-0 list-none flex-1 min-h-0 overflow-y-auto">
          {results.length === 0 ? (
            <li className="px-4 py-6 text-sm text-secondary text-center">
              {t("conversations.noSessions")}
            </li>
          ) : (
            results.map((session) => (
              <li key={session.id}>
                <button
                  type="button"
                  className="w-full text-left px-4 py-2.5 border-0 bg-transparent cursor-pointer hover:bg-surface-container-low flex items-start gap-2 min-w-0"
                  onClick={() => {
                    onSelect(session.id);
                    onClose();
                  }}
                >
                  <Icon name="forum" size={16} className="text-secondary shrink-0 mt-0.5" />
                  <span className="min-w-0 flex-1">
                    <span className="text-sm font-medium truncate block">
                      {session.title || session.id}
                    </span>
                    <span className="text-xs text-secondary truncate block mt-0.5">
                      {session.project_name || t("home.hero.noProject")}
                    </span>
                  </span>
                  <span className="text-xs text-secondary tabular-nums shrink-0">
                    {formatRelativeTime(session.started_at)}
                  </span>
                </button>
              </li>
            ))
          )}
        </ul>
      </div>
    </ModalOverlay>
  );
}
