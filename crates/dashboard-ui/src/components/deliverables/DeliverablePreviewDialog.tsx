import type { ReactNode } from "react";
import { Icon } from "@/components/Icon";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useT } from "@/i18n/context";

type Props = {
  open: boolean;
  onClose: () => void;
  title: string;
  subtitle?: string;
  children: ReactNode;
  /** Wider modal for diagrams / reports */
  wide?: boolean;
};

export function DeliverablePreviewDialog({
  open,
  onClose,
  title,
  subtitle,
  children,
  wide = false,
}: Props) {
  const t = useT();
  const titleId = "deliverable-preview-title";

  return (
    <ModalOverlay
      open={open}
      onClose={onClose}
      labelledBy={titleId}
      className={`w-full ${wide ? "max-w-5xl" : "max-w-3xl"}`}
      zIndex={360}
    >
      <div className="deliverable-preview-dialog glass-modal rounded-2xl overflow-hidden flex flex-col max-h-[min(90dvh,880px)]">
        <div className="deliverable-preview-dialog__head shrink-0 px-4 py-3 border-b border-outline-variant/40 flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h2 id={titleId} className="m-0 text-base font-semibold truncate">
              {title}
            </h2>
            {subtitle ? (
              <p className="m-0 mt-0.5 text-xs text-secondary truncate">{subtitle}</p>
            ) : null}
          </div>
          <button
            type="button"
            className="dw-btn-ghost p-1.5 shrink-0"
            onClick={onClose}
            aria-label={t("common.close")}
          >
            <Icon name="close" size={18} />
          </button>
        </div>
        <div className="deliverable-preview-dialog__body flex-1 min-h-0 overflow-auto">
          {children}
        </div>
      </div>
    </ModalOverlay>
  );
}
