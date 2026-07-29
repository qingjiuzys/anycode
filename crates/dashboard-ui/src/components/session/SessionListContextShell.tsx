import { useEffect, useRef, useState } from "react";
import { useT } from "@/i18n/context";

type MenuState = {
  sessionId: string;
  x: number;
  y: number;
};

type Props = {
  onRename: (sessionId: string, title: string) => void;
  onHandoffToColleague?: (sessionId: string) => void;
  children: (handlers: {
    onContextMenu: (sessionId: string, event: React.MouseEvent) => void;
    renamingSessionId: string | null;
    onRenameSave: (sessionId: string, title: string) => void;
    onRenameCancel: () => void;
  }) => React.ReactNode;
};

export function SessionListContextShell({ onRename, onHandoffToColleague, children }: Props) {
  const t = useT();
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [renamingSessionId, setRenamingSessionId] = useState<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menu) return;
    const onPointerDown = (event: MouseEvent) => {
      if (menuRef.current?.contains(event.target as Node)) {
        return;
      }
      setMenu(null);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenu(null);
      }
    };
    window.addEventListener("mousedown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [menu]);

  const onContextMenu = (sessionId: string, event: React.MouseEvent) => {
    event.preventDefault();
    event.stopPropagation();
    setMenu({ sessionId, x: event.clientX, y: event.clientY });
  };

  return (
    <>
      {children({
        onContextMenu,
        renamingSessionId,
        onRenameSave: (sessionId, title) => {
          onRename(sessionId, title);
          setRenamingSessionId(null);
        },
        onRenameCancel: () => setRenamingSessionId(null),
      })}
      {menu && (
        <div
          ref={menuRef}
          className="fixed z-[100] min-w-[10rem] rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg py-1"
          style={{ left: menu.x, top: menu.y }}
          role="menu"
        >
          <button
            type="button"
            role="menuitem"
            className="w-full text-left px-3 py-2 text-sm border-0 bg-transparent hover:bg-surface-container-low cursor-pointer"
            onClick={() => {
              setRenamingSessionId(menu.sessionId);
              setMenu(null);
            }}
          >
            {t("conversations.renameSession")}
          </button>
          {onHandoffToColleague ? (
            <button
              type="button"
              role="menuitem"
              className="w-full text-left px-3 py-2 text-sm border-0 bg-transparent hover:bg-surface-container-low cursor-pointer"
              onClick={() => {
                onHandoffToColleague(menu.sessionId);
                setMenu(null);
              }}
            >
              {t("conversations.handoffToColleague")}
            </button>
          ) : null}
        </div>
      )}
    </>
  );
}

export function SessionRenameInput({
  initialTitle,
  label,
  onSave,
  onCancel,
}: {
  initialTitle: string;
  label: string;
  onSave: (title: string) => void;
  onCancel: () => void;
}) {
  const [draft, setDraft] = useState(initialTitle);
  const cancelledRef = useRef(false);

  return (
    <input
      // eslint-disable-next-line jsx-a11y/no-autofocus
      autoFocus
      className="dw-input text-sm w-full"
      value={draft}
      maxLength={120}
      aria-label={label}
      onChange={(e) => setDraft(e.target.value)}
      onFocus={(e) => e.target.select()}
      onBlur={() => {
        if (cancelledRef.current) {
          cancelledRef.current = false;
          onCancel();
          return;
        }
        const trimmed = draft.trim();
        if (trimmed) {
          onSave(trimmed);
        } else {
          onCancel();
        }
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          cancelledRef.current = true;
          e.currentTarget.blur();
        }
      }}
      onClick={(e) => e.stopPropagation()}
    />
  );
}
