import { useEffect, useMemo, useState } from "react";
import type { SessionWithProject } from "@/api/types";
import { Icon } from "@/components/Icon";
import {
  SessionRenameInput,
  SessionListContextShell,
} from "@/components/session/SessionListContextShell";
import {
  SessionRunningDots,
  SessionStatusBadges,
} from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";
import { groupSessionsByProject } from "@/lib/groupSessionsByProject";
import { formatRelativeTime } from "@/utils/formatTime";

const DEFAULT_EXPANDED_COUNT = 2;

type Props = {
  projectOptions: Array<{ id: string; name: string; updated_at?: string }>;
  sessions: SessionWithProject[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  pendingCounts?: Map<string, number>;
  onPrefetch?: (sessionId: string, isRunning: boolean) => void;
  hideEmptyProjects?: boolean;
  onNewSession?: (projectId: string) => void;
  activeProjectId?: string;
  onSelectProject?: (projectId: string) => void;
  onRenameSession?: (sessionId: string, title: string) => void;
  optimisticStreamingSessionId?: string | null;
};

function statusDotClass(status: string, trusted: string, runningVisual: boolean): string {
  if (trusted === "blocked") return "bg-error";
  if (runningVisual) return "bg-primary animate-pulse";
  if (status === "failed") return "bg-error";
  if (status === "completed") return "bg-secondary";
  return "bg-outline";
}

function sessionRunningVisual(
  session: SessionWithProject,
  optimisticStreamingSessionId?: string | null,
): boolean {
  return session.status === "running" || optimisticStreamingSessionId === session.id;
}

export function ProjectGroupedSessionList({
  projectOptions,
  sessions,
  selectedId,
  onSelect,
  pendingCounts,
  onPrefetch,
  hideEmptyProjects = false,
  onNewSession,
  activeProjectId,
  onSelectProject,
  onRenameSession,
  optimisticStreamingSessionId = null,
}: Props) {
  const t = useT();
  const groups = useMemo(() => {
    const grouped = groupSessionsByProject(projectOptions, sessions);
    if (!hideEmptyProjects) return grouped;
    return grouped.filter((group) => group.sessions.length > 0);
  }, [hideEmptyProjects, projectOptions, sessions]);

  const [collapsedOverride, setCollapsedOverride] = useState<Set<string>>(() => new Set());
  const [expandedOverride, setExpandedOverride] = useState<Set<string>>(() => new Set());

  const isCollapsed = (projectId: string, index: number) => {
    if (expandedOverride.has(projectId)) return false;
    if (collapsedOverride.has(projectId)) return true;
    return index >= DEFAULT_EXPANDED_COUNT;
  };

  useEffect(() => {
    if (!selectedId) return;
    const selected = sessions.find((session) => session.id === selectedId);
    if (!selected) return;
    setExpandedOverride((prev) => {
      if (prev.has(selected.project_id)) return prev;
      const next = new Set(prev);
      next.add(selected.project_id);
      return next;
    });
    setCollapsedOverride((prev) => {
      if (!prev.has(selected.project_id)) return prev;
      const next = new Set(prev);
      next.delete(selected.project_id);
      return next;
    });
  }, [selectedId, sessions]);

  if (groups.length === 0) {
    return (
      <p className="text-sm text-secondary px-3 py-4 m-0">{t("conversations.noSessionsAny")}</p>
    );
  }

  function toggleProject(projectId: string, index: number) {
    const collapsed = isCollapsed(projectId, index);
    if (collapsed) {
      setExpandedOverride((prev) => new Set(prev).add(projectId));
      setCollapsedOverride((prev) => {
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
      return;
    }
    setCollapsedOverride((prev) => new Set(prev).add(projectId));
    setExpandedOverride((prev) => {
      const next = new Set(prev);
      next.delete(projectId);
      return next;
    });
  }

  function expandProject(projectId: string) {
    setExpandedOverride((prev) => new Set(prev).add(projectId));
    setCollapsedOverride((prev) => {
      const next = new Set(prev);
      next.delete(projectId);
      return next;
    });
  }

  function openNewSession(projectId: string) {
    expandProject(projectId);
    onNewSession?.(projectId);
  }

  const renderGroups = (
    ctx?: {
      onContextMenu: (sessionId: string, event: React.MouseEvent) => void;
      renamingSessionId: string | null;
      onRenameSave: (sessionId: string, title: string) => void;
      onRenameCancel: () => void;
    },
  ) => (
    <div className="py-1">
      {groups.map((group, index) => {
        const collapsed = isCollapsed(group.id, index);
        const countLabel = t("conversations.projectSessionCount").replace(
          "{n}",
          String(group.sessions.length),
        );
        const projectActive = activeProjectId === group.id;
        return (
          <section key={group.id} className="dw-project-session-group">
            <div className="dw-project-session-group__head">
              <div className="dw-project-session-group__toggle">
                <button
                  type="button"
                  className="dw-project-session-group__chevron"
                  aria-expanded={!collapsed}
                  aria-label={group.name}
                  onClick={() => toggleProject(group.id, index)}
                >
                  <Icon
                    name={collapsed ? "chevron_right" : "expand_more"}
                    size={16}
                    className="text-secondary shrink-0"
                  />
                </button>
                <button
                  type="button"
                  className={`dw-project-session-group__name-btn truncate${projectActive ? " dw-project-session-group__name-btn--active" : ""}`}
                  title={group.name}
                  onClick={() => {
                    expandProject(group.id);
                    onSelectProject?.(group.id);
                  }}
                >
                  {group.name}
                </button>
              </div>
              <span className="dw-project-session-group__count">{countLabel}</span>
              {onNewSession && (
                <button
                  type="button"
                  className="dw-project-session-group__add"
                  aria-label={t("conversations.newSession")}
                  title={t("conversations.newSession")}
                  onClick={() => openNewSession(group.id)}
                >
                  <Icon name="add" size={16} />
                </button>
              )}
            </div>
            {!collapsed && (
              <SessionRows
                sessions={group.sessions}
                selectedId={selectedId}
                onSelect={onSelect}
                pendingCounts={pendingCounts}
                onPrefetch={onPrefetch}
                optimisticStreamingSessionId={optimisticStreamingSessionId}
                onRenameSession={onRenameSession}
                contextMenu={ctx?.onContextMenu}
                renamingSessionId={ctx?.renamingSessionId}
                onRenameSave={ctx?.onRenameSave}
                onRenameCancel={ctx?.onRenameCancel}
              />
            )}
          </section>
        );
      })}
    </div>
  );

  if (!onRenameSession) {
    return renderGroups();
  }

  return (
    <SessionListContextShell onRename={onRenameSession}>
      {(ctx) => renderGroups(ctx)}
    </SessionListContextShell>
  );
}

function SessionRows({
  sessions,
  selectedId,
  onSelect,
  pendingCounts,
  onPrefetch,
  optimisticStreamingSessionId,
  onRenameSession,
  contextMenu,
  renamingSessionId,
  onRenameSave,
  onRenameCancel,
}: {
  sessions: SessionWithProject[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  pendingCounts?: Map<string, number>;
  onPrefetch?: (sessionId: string, isRunning: boolean) => void;
  optimisticStreamingSessionId?: string | null;
  onRenameSession?: (sessionId: string, title: string) => void;
  contextMenu?: (sessionId: string, event: React.MouseEvent) => void;
  renamingSessionId?: string | null;
  onRenameSave?: (sessionId: string, title: string) => void;
  onRenameCancel?: () => void;
}) {
  const t = useT();

  if (sessions.length === 0) {
    return (
      <ul className="m-0 p-0 list-none">
        <li className="px-3 py-2 pl-7 text-xs text-secondary">{t("conversations.noSessions")}</li>
      </ul>
    );
  }

  return (
    <ul className="m-0 p-0 list-none">
      {sessions.map((session) => {
        const active = session.id === selectedId;
        const pending = pendingCounts?.get(session.id) ?? 0;
        const runningVisual = sessionRunningVisual(session, optimisticStreamingSessionId);
        const showAlertBadge =
          session.status === "failed" ||
          session.trusted_status === "blocked" ||
          pending > 0;
        const renaming = renamingSessionId === session.id;

        return (
          <li key={session.id} className="group">
            <button
              type="button"
              onClick={() => onSelect(session.id)}
              onContextMenu={
                onRenameSession && contextMenu
                  ? (event) => contextMenu(session.id, event)
                  : undefined
              }
              onMouseEnter={() => onPrefetch?.(session.id, session.status === "running")}
              onFocus={() => onPrefetch?.(session.id, session.status === "running")}
              className={`w-full text-left pl-7 pr-3 py-2 border-0 cursor-pointer transition-colors flex items-start gap-2 min-w-0 ${
                active
                  ? "bg-surface-container-high"
                  : "bg-transparent hover:bg-surface-container-low"
              }${runningVisual ? " dw-session-row--running" : ""}`}
            >
              <span
                className={`shrink-0 w-2 h-2 rounded-full mt-1.5 ${statusDotClass(session.status, session.trusted_status, runningVisual)}`}
                title={session.status}
              />
              <span className="min-w-0 flex-1">
                {renaming && onRenameSave && onRenameCancel ? (
                  <SessionRenameInput
                    initialTitle={session.title || session.id}
                    label={t("conversations.renameSession")}
                    onSave={(title) => onRenameSave(session.id, title)}
                    onCancel={onRenameCancel}
                  />
                ) : (
                  <span className="text-sm font-medium truncate block">
                    {session.title || session.id}
                  </span>
                )}
                {pending > 0 && (
                  <span className="text-[11px] text-warn truncate block mt-0.5">
                    {t("home.securityPendingBadge").replace("{n}", String(pending))}
                  </span>
                )}
              </span>
              <span className="shrink-0 flex flex-col items-end gap-1 pt-0.5 min-w-[2.75rem]">
                {runningVisual ? (
                  <SessionRunningDots />
                ) : (
                  <span className="text-[11px] text-secondary tabular-nums">
                    {formatRelativeTime(session.started_at)}
                  </span>
                )}
                {showAlertBadge && (
                  <SessionStatusBadges
                    variant="sidebar"
                    status={session.status}
                    trustedStatus={session.trusted_status}
                    pendingApprovalCount={pending}
                  />
                )}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
