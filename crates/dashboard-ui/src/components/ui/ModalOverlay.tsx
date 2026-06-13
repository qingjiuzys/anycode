import { useEffect, type ReactNode } from "react";
import { createPortal } from "react-dom";

type Props = {
  open: boolean;
  onClose: () => void;
  /** id of the visible title element for aria-labelledby */
  labelledBy?: string;
  /** Close when clicking the backdrop (default true) */
  dismissOnBackdrop?: boolean;
  children: ReactNode;
  className?: string;
};

/** Viewport-centered modal shell portaled to document.body (avoids topbar backdrop-filter breaking fixed). */
export function ModalOverlay({
  open,
  onClose,
  labelledBy,
  dismissOnBackdrop = true,
  children,
  className = "w-full max-w-lg",
}: Props) {
  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[200] flex items-center justify-center p-4 pt-[max(1rem,env(safe-area-inset-top))] pb-[max(1rem,env(safe-area-inset-bottom))] bg-on-surface/30 backdrop-blur-[2px]"
      role="dialog"
      aria-modal
      aria-labelledby={labelledBy}
      onClick={dismissOnBackdrop ? onClose : undefined}
    >
      <div className={className} onClick={(e) => e.stopPropagation()}>
        {children}
      </div>
    </div>,
    document.body,
  );
}
