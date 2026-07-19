import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import type { OptimisticQueueItem } from "@/lib/optimisticMessageQueue";

type Props = {
  items: OptimisticQueueItem[];
  cancelling?: boolean;
  onCancel: (queueId: string) => void;
};

/**
 * Cursor-style follow-up queue card above the composer.
 * Multitasking CTA is intentionally omitted (UI-only pass).
 */
export function FollowUpQueueCard({ items, cancelling, onCancel }: Props) {
  const t = useT();
  if (items.length === 0) return null;

  const countLabel = t("conversations.messageQueueCount").replace(
    "{n}",
    String(items.length),
  );

  return (
    <div className="dw-followup-queue" role="status" aria-live="polite">
      <div className="dw-followup-queue__header">
        <div className="dw-followup-queue__header-start">
          <span className="dw-followup-queue__count">{countLabel}</span>
          <span className="dw-followup-queue__to-send">
            <Icon name="keyboard_return" size={14} />
            <span>{t("conversations.messageQueueToSend")}</span>
          </span>
        </div>
      </div>
      <ul className="dw-followup-queue__list">
        {items.map((item) => (
          <li key={item.id} className="dw-followup-queue__item">
            <span className="dw-followup-queue__marker" aria-hidden />
            <span className="dw-followup-queue__prompt" title={item.prompt}>
              {item.prompt}
            </span>
            <button
              type="button"
              className="dw-followup-queue__cancel"
              title={t("conversations.messageQueueCancel")}
              aria-label={t("conversations.messageQueueCancel")}
              disabled={cancelling}
              onClick={() => onCancel(item.id)}
            >
              <Icon name="close" size={14} />
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
