import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api } from "@/api/client";
import type { SessionWithProject } from "@/api/types";
import { Icon } from "@/components/Icon";
import { SessionRenameInput } from "@/components/session/SessionListContextShell";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useT } from "@/i18n/context";

type Props = {
  session: SessionWithProject;
  onRename?: (sessionId: string, title: string) => void | Promise<void>;
};

export function SessionTitleMenu({ session, onRename }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [jsonOpen, setJsonOpen] = useState(false);
  const [jsonText, setJsonText] = useState("");
  const [jsonLoading, setJsonLoading] = useState(false);
  const [copyHint, setCopyHint] = useState<string | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const listId = useId();
  const [menuPos, setMenuPos] = useState<{ left: number; top: number } | null>(null);

  useEffect(() => {
    if (!open) return;
    const update = () => {
      const el = triggerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const width = 240;
      const left = Math.max(12, Math.min(rect.left + rect.width / 2 - width / 2, window.innerWidth - width - 12));
      setMenuPos({ left, top: rect.bottom + 8 });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      const target = e.target as Node;
      if (menuRef.current?.contains(target) || triggerRef.current?.contains(target)) return;
      setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  useEffect(() => {
    if (!copyHint) return;
    const id = window.setTimeout(() => setCopyHint(null), 2000);
    return () => window.clearTimeout(id);
  }, [copyHint]);

  const copyConversation = async () => {
    setOpen(false);
    try {
      const { transcript } = await api.sessionTranscript(session.id);
      const lines = (transcript.blocks ?? [])
        .map((b) => {
          const body = (b.body || "").trim();
          if (!body) return null;
          const label = b.title || b.block_type || "message";
          return `[${label}]\n${body}`;
        })
        .filter(Boolean);
      const text = [`# ${session.title || session.id}`, "", ...lines].join("\n\n");
      await navigator.clipboard.writeText(text);
      setCopyHint(t("conversations.copySessionDone"));
    } catch {
      const fallback = `${session.title || ""}\n${session.id}`;
      await navigator.clipboard.writeText(fallback);
      setCopyHint(t("conversations.copySessionDone"));
    }
  };

  const viewJson = async () => {
    setOpen(false);
    setJsonOpen(true);
    setJsonLoading(true);
    try {
      const [detail, transcriptRes] = await Promise.all([
        api.session(session.id),
        api.sessionTranscript(session.id).catch(() => null),
      ]);
      setJsonText(
        JSON.stringify(
          {
            session: detail.session,
            transcript: transcriptRes?.transcript ?? null,
          },
          null,
          2,
        ),
      );
    } catch (err) {
      setJsonText(
        JSON.stringify(
          {
            session,
            error: err instanceof Error ? err.message : String(err),
          },
          null,
          2,
        ),
      );
    } finally {
      setJsonLoading(false);
    }
  };

  if (renaming && onRename) {
    return (
      <div className="conv-title-menu conv-title-menu--renaming min-w-0 max-w-md w-full">
        <SessionRenameInput
          initialTitle={session.title || ""}
          label={t("conversations.renameSession")}
          onSave={async (title) => {
            await onRename(session.id, title);
            setRenaming(false);
          }}
          onCancel={() => setRenaming(false)}
        />
      </div>
    );
  }

  return (
    <>
      <div className="conv-title-menu relative min-w-0 max-w-full w-full">
        <button
          ref={triggerRef}
          type="button"
          className="conv-title-menu__trigger max-w-full"
          aria-haspopup="menu"
          aria-expanded={open}
          aria-controls={listId}
          onClick={() => setOpen((v) => !v)}
        >
          <span className="conv-title-menu__title truncate">{session.title || session.id}</span>
          <Icon name="expand_more" size={18} className="conv-title-menu__chev shrink-0" />
        </button>
        {copyHint ? <p className="conv-title-menu__hint m-0">{copyHint}</p> : null}
      </div>

      {open &&
        menuPos &&
        createPortal(
          <div
            ref={menuRef}
            id={listId}
            className="dw-project-menu conv-title-menu__popup"
            style={{ left: menuPos.left, top: menuPos.top, minWidth: "15rem" }}
            role="menu"
          >
            {onRename ? (
              <button
                type="button"
                role="menuitem"
                className="dw-project-menu__item"
                onClick={() => {
                  setOpen(false);
                  setRenaming(true);
                }}
              >
                <Icon name="edit" size={16} />
                <span className="dw-project-menu__label">{t("conversations.renameSession")}</span>
              </button>
            ) : null}
            <button
              type="button"
              role="menuitem"
              className="dw-project-menu__item"
              onClick={() => void copyConversation()}
            >
              <Icon name="content_copy" size={16} />
              <span className="dw-project-menu__label">{t("conversations.copySession")}</span>
            </button>
            <button
              type="button"
              role="menuitem"
              className="dw-project-menu__item"
              onClick={() => void viewJson()}
            >
              <Icon name="code" size={16} />
              <span className="dw-project-menu__label">{t("conversations.viewSessionJson")}</span>
            </button>
          </div>,
          document.body,
        )}

      <ModalOverlay
        open={jsonOpen}
        onClose={() => setJsonOpen(false)}
        labelledBy="session-json-title"
        className="w-full max-w-2xl"
      >
        <div className="glass-modal rounded-xl p-5 flex flex-col max-h-[min(90dvh,720px)]">
          <div className="flex items-start justify-between gap-3 mb-3 shrink-0">
            <h2 id="session-json-title" className="text-lg font-semibold m-0">
              {t("conversations.viewSessionJson")}
            </h2>
            <button
              type="button"
              className="dw-btn-ghost p-1"
              onClick={() => setJsonOpen(false)}
              aria-label={t("controlCenter.close")}
            >
              <Icon name="close" size={20} />
            </button>
          </div>
          <pre className="m-0 flex-1 min-h-0 overflow-auto text-xs font-code rounded-lg border border-outline-variant bg-surface-container-low p-3">
            {jsonLoading ? t("common.loading") : jsonText}
          </pre>
        </div>
      </ModalOverlay>
    </>
  );
}
