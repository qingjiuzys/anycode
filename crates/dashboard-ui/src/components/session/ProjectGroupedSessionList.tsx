import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
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
import {
  groupSessionsByProject,
  type ProjectGroupOption,
} from "@/lib/groupSessionsByProject";
import { revealInFileManager } from "@/lib/openExternal";
import {
  readPinnedProjectIds,
  togglePinnedProjectId,
} from "@/lib/pinnedProjects";
import { formatRelativeTime } from "@/utils/formatTime";

const DEFAULT_EXPANDED_COUNT = 2;

type Props = {
  projectOptions: ProjectGroupOption[];
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
  onRenameProject?: (projectId: string, name: string) => void;
  onRemoveProject?: (projectId: string) => void;
  optimisticStreamingSessionId?: string | null;
};

type ProjectMenuState = {
  projectId: string;
  x: number;
  y: number;
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
  onRenameProject,
  onRemoveProject,
  optimisticStreamingSessionId = null,
}: Props) {
  const t = useT();
  const [pinnedIds, setPinnedIds] = useState<string[]>(() => readPinnedProjectIds());
  const pinnedSet = useMemo(() => new Set(pinnedIds), [pinnedIds]);
  const [menu, setMenu] = useState<ProjectMenuState | null>(null);
  const [renamingProjectId, setRenamingProjectId] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const groups = useMemo(() => {
    const grouped = groupSessionsByProject(projectOptions, sessions, pinnedSet);
    if (!hideEmptyProjects) return grouped;
    return grouped.filter((group) => group.sessions.length > 0);
  }, [hideEmptyProjects, pinnedSet, projectOptions, sessions]);

  const projectById = useMemo(() => {
    const map = new Map(projectOptions.map((project) => [project.id, project]));
    return map;
  }, [projectOptions]);

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

  useEffect(() => {
    if (!menu) return;
    // Use click (not mousedown) so menu item handlers can run before dismiss.
    const onClick = (event: MouseEvent) => {
      if (menuRef.current?.contains(event.target as Node)) return;
      setMenu(null);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setMenu(null);
    };
    window.addEventListener("click", onClick, true);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("click", onClick, true);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [menu]);

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

  function openProjectMenu(projectId: string, event: React.MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const menuWidth = 220;
    const menuHeight = 280;
    // Anchor under the "…" button, left-aligned (match product mock).
    let x = rect.left;
    let y = rect.bottom + 4;
    if (x + menuWidth > window.innerWidth - 8) {
      x = Math.max(8, window.innerWidth - menuWidth - 8);
    }
    if (x < 8) x = 8;
    if (y + menuHeight > window.innerHeight - 8) {
      y = Math.max(8, rect.top - menuHeight - 4);
    }
    setMenu({ projectId, x, y });
  }

  const menuProject = menu ? projectById.get(menu.projectId) : undefined;

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
        const projectActive = activeProjectId === group.id;
        const projectMeta = projectById.get(group.id);
        const renaming = renamingProjectId === group.id;
        return (
          <section key={group.id} className="dw-project-session-group">
            <div className="dw-project-session-group__head">
              <div className="dw-project-session-group__toggle">
                {renaming && onRenameProject ? (
                  <SessionRenameInput
                    initialTitle={group.name}
                    label={t("conversations.renameProject")}
                    onSave={(name) => {
                      onRenameProject(group.id, name);
                      setRenamingProjectId(null);
                    }}
                    onCancel={() => setRenamingProjectId(null)}
                  />
                ) : (
                  <button
                    type="button"
                    className={`dw-project-session-group__name-btn truncate${projectActive ? " dw-project-session-group__name-btn--active" : ""}`}
                    title={projectMeta?.root_path || group.name}
                    aria-expanded={!collapsed}
                    onClick={() => {
                      toggleProject(group.id, index);
                      onSelectProject?.(group.id);
                    }}
                  >
                    <Icon name="folder" size={16} className="shrink-0 text-secondary" />
                    {pinnedSet.has(group.id) && (
                      <Icon name="star" size={14} className="shrink-0 text-primary" filled />
                    )}
                    <span className="truncate">{group.name}</span>
                  </button>
                )}
              </div>
              <button
                type="button"
                className="dw-project-session-group__chevron"
                aria-expanded={!collapsed}
                aria-label={
                  collapsed
                    ? t("common.expand")
                    : t("common.collapse")
                }
                onClick={(event) => {
                  event.stopPropagation();
                  toggleProject(group.id, index);
                  // Avoid focus-within leaving the chevron permanently visible after click.
                  (event.currentTarget as HTMLButtonElement).blur();
                }}
              >
                <Icon
                  name={collapsed ? "chevron_right" : "expand_more"}
                  size={16}
                  className="text-secondary shrink-0"
                />
              </button>
              <button
                type="button"
                className="dw-project-session-group__menu"
                aria-label={t("conversations.projectMenu")}
                title={t("conversations.projectMenu")}
                onClick={(event) => openProjectMenu(group.id, event)}
              >
                <Icon name="more_horiz" size={16} />
              </button>
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

      {menu &&
        createPortal(
          <div
            ref={menuRef}
            className="dw-project-menu"
            style={{ left: menu.x, top: menu.y }}
            role="menu"
          >
            {onNewSession && (
              <button
                type="button"
                role="menuitem"
                className="dw-project-menu__item"
                onClick={() => {
                  openNewSession(menu.projectId);
                  setMenu(null);
                }}
              >
                <Icon name="add" size={16} />
                <span className="dw-project-menu__label">{t("conversations.newSession")}</span>
              </button>
            )}
            {onRenameProject && (
              <button
                type="button"
                role="menuitem"
                className="dw-project-menu__item"
                onClick={() => {
                  setRenamingProjectId(menu.projectId);
                  setMenu(null);
                }}
              >
                <Icon name="edit" size={16} />
                <span className="dw-project-menu__label">{t("conversations.renameProject")}</span>
              </button>
            )}
            <button
              type="button"
              role="menuitem"
              className="dw-project-menu__item"
              onClick={() => {
                setPinnedIds(togglePinnedProjectId(menu.projectId));
                setMenu(null);
              }}
            >
              <Icon
                name="star"
                size={16}
                filled={pinnedSet.has(menu.projectId)}
              />
              <span className="dw-project-menu__label">
                {pinnedSet.has(menu.projectId)
                  ? t("conversations.unpinProject")
                  : t("conversations.pinProject")}
              </span>
            </button>
            {onRemoveProject && (
              <button
                type="button"
                role="menuitem"
                className="dw-project-menu__item"
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                const id = menu.projectId;
                const name = menuProject?.name ?? id;
                setMenu(null);
                if (
                  window.confirm(
                    t("conversations.removeProjectConfirm").replace("{name}", name),
                  )
                ) {
                  onRemoveProject(id);
                }
              }}
              >
                <Icon name="close" size={16} />
                <span className="dw-project-menu__label">{t("conversations.removeProject")}</span>
              </button>
            )}
            {menuProject?.root_path && (
              <>
                <button
                  type="button"
                  role="menuitem"
                  className="dw-project-menu__item"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  const path = menuProject.root_path!;
                  setMenu(null);
                  void revealInFileManager(path).catch((err) => {
                    window.alert(
                      err instanceof Error
                        ? err.message
                        : t("conversations.openInFinderFailed"),
                    );
                  });
                }}
                >
                  <Icon name="folder_open" size={16} />
                  <span className="dw-project-menu__label">{t("conversations.openInFinder")}</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  className="dw-project-menu__item"
                  onClick={() => {
                    void navigator.clipboard.writeText(menuProject.root_path!);
                    setMenu(null);
                  }}
                >
                  <Icon name="content_copy" size={16} />
                  <span className="dw-project-menu__label">
                    {t("conversations.copyProjectPath")}
                  </span>
                </button>
              </>
            )}
          </div>,
          document.body,
        )}
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
        <li className="px-3 py-2 pl-7 text-sm text-secondary">{t("conversations.noSessions")}</li>
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
                  <span className="text-[14px] font-medium truncate block leading-snug">
                    {session.title || session.id}
                  </span>
                )}
                {pending > 0 && (
                  <span className="text-xs text-warn truncate block mt-0.5">
                    {t("home.securityPendingBadge").replace("{n}", String(pending))}
                  </span>
                )}
              </span>
              <span className="shrink-0 flex flex-col items-end gap-1 pt-0.5 min-w-[2.75rem]">
                {runningVisual ? (
                  <SessionRunningDots />
                ) : (
                  <span className="text-xs text-secondary tabular-nums">
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
