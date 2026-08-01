import { useEffect, useRef } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  x: number;
  y: number;
  onClose: () => void;
  onReveal: () => void;
  onCopyPath: () => void;
};

export function DeliverableContextMenu({ x, y, onClose, onReveal, onCopyPath }: Props) {
  const t = useT();
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const onDoc = (event: MouseEvent) => {
      if (ref.current?.contains(event.target as Node)) return;
      onClose();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className="dw-deliverable-context-menu"
      style={{ left: x, top: y }}
      role="menu"
    >
      <button
        type="button"
        role="menuitem"
        className="dw-deliverable-context-menu__item"
        onClick={() => {
          onClose();
          onReveal();
        }}
      >
        <Icon name="folder_open" size={16} />
        <span>{t("conversations.openInFinder")}</span>
      </button>
      <button
        type="button"
        role="menuitem"
        className="dw-deliverable-context-menu__item"
        onClick={() => {
          onClose();
          onCopyPath();
        }}
      >
        <Icon name="content_copy" size={16} />
        <span>{t("conversations.deliverable.copyPath")}</span>
      </button>
    </div>
  );
}
