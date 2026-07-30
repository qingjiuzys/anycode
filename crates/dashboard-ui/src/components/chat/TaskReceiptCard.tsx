import { memo } from "react";
import { TranscriptMarkdown } from "@/components/TranscriptMarkdown";
import { stripTaskReceiptHeading } from "@/lib/taskSummaryReceipt";
import { useT } from "@/i18n/context";

type Props = {
  body: string;
  live?: boolean;
  showCursor?: boolean;
};

export const TaskReceiptCard = memo(function TaskReceiptCard({
  body,
  live = false,
  showCursor = false,
}: Props) {
  const t = useT();
  const content = stripTaskReceiptHeading(body);

  return (
    <article className="task-receipt-card" data-testid="task-receipt-card">
      <header className="task-receipt-card__header">{t("conversations.assistantReply")}</header>
      <div className="task-receipt-card__body">
        <h3 className="task-receipt-card__title">{t("conversations.taskReceiptTitle")}</h3>
        <TranscriptMarkdown text={content} live={live} />
        {showCursor && <span className="chat-stream-cursor" aria-hidden />}
      </div>
    </article>
  );
});
