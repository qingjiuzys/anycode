import { useMutation } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import type { LivePendingQuestion } from "@/lib/sessionLiveStore";

type Props = {
  sessionId: string;
  questions: LivePendingQuestion[];
  respondAllowed: boolean;
  hideWhenEmpty?: boolean;
  inline?: boolean;
};

export function AskUserQuestionInbox({
  sessionId: _sessionId,
  questions,
  respondAllowed,
  hideWhenEmpty = false,
  inline = false,
}: Props) {
  const t = useT();
  const [otherText, setOtherText] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<Record<string, Set<string>>>({});
  const [removedIds, setRemovedIds] = useState<Set<string>>(new Set());

  const respond = useMutation({
    mutationFn: ({
      questionId,
      selected_labels,
      other_text,
    }: {
      questionId: string;
      selected_labels: string[];
      other_text?: string;
    }) => api.respondToQuestion(questionId, { selected_labels, other_text }),
    onMutate: ({ questionId }) => {
      setRemovedIds((prev) => new Set(prev).add(questionId));
    },
    onError: (_err, { questionId }) => {
      setRemovedIds((prev) => {
        const next = new Set(prev);
        next.delete(questionId);
        return next;
      });
    },
  });

  const rows = questions.filter((row) => !removedIds.has(row.question_id));
  const canRespond = respondAllowed;

  if (rows.length === 0) {
    return hideWhenEmpty ? null : null;
  }

  const rowList = (
    <div className={inline ? "space-y-2" : "space-y-3"}>
      {rows.map((row) => {
        const sel = selected[row.question_id] ?? new Set<string>();
        const toggle = (label: string) => {
          setSelected((prev) => {
            const next = new Set(prev[row.question_id] ?? []);
            if (row.multi_select) {
              if (next.has(label)) next.delete(label);
              else next.add(label);
            } else {
              next.clear();
              next.add(label);
            }
            return { ...prev, [row.question_id]: next };
          });
        };
        const submit = () => {
          const labels = [...(selected[row.question_id] ?? [])];
          const other = otherText[row.question_id]?.trim();
          respond.mutate({
            questionId: row.question_id,
            selected_labels: labels,
            other_text: other || undefined,
          });
        };
        return (
          <div
            key={row.question_id}
            className={
              inline
                ? "rounded-xl border border-primary/30 bg-primary/5 p-3"
                : "rounded-lg border border-primary/30 bg-surface-container-lowest p-3"
            }
          >
            <div className="flex items-start gap-2 mb-2">
              <Icon name="quiz" size={18} className="text-primary shrink-0 mt-0.5" />
              <div className="min-w-0 flex-1">
                {row.header && (
                  <p className="text-[10px] font-semibold uppercase tracking-wide text-secondary m-0 mb-0.5">
                    {row.header}
                  </p>
                )}
                <p className="text-sm font-medium text-on-surface m-0">{row.question}</p>
              </div>
            </div>
            <div className="flex flex-col gap-1.5 mb-3">
              {row.options.map((opt) => {
                const active = sel.has(opt.label);
                return (
                  <button
                    key={opt.label}
                    type="button"
                    disabled={!canRespond || respond.isPending}
                    onClick={() => toggle(opt.label)}
                    className={`text-left px-3 py-2 rounded-md border text-sm transition-colors ${
                      active
                        ? "border-primary bg-primary/10 text-on-surface"
                        : "border-outline-variant bg-surface-container-low hover:bg-surface-container"
                    }`}
                  >
                    <span className="font-medium">{opt.label}</span>
                    {opt.description && (
                      <span className="block text-xs text-secondary mt-0.5">{opt.description}</span>
                    )}
                  </button>
                );
              })}
            </div>
            <input
              type="text"
              className="dw-input w-full text-sm mb-2"
              placeholder={t("conversations.askOtherPlaceholder")}
              value={otherText[row.question_id] ?? ""}
              disabled={!canRespond || respond.isPending}
              onChange={(e) =>
                setOtherText((prev) => ({ ...prev, [row.question_id]: e.target.value }))
              }
            />
            <button
              type="button"
              className="dw-btn-primary text-xs"
              disabled={!canRespond || respond.isPending}
              onClick={submit}
            >
              {t("conversations.askSubmit")}
            </button>
          </div>
        );
      })}
    </div>
  );

  if (inline) {
    return (
      <div className="chat-trace chat-trace-question">
        <div className="chat-trace-toggle-static">
          <span className="inline-flex items-center gap-1.5 text-primary">
            <Icon name="quiz" size={16} />
            {t("conversations.askInboxTitle")}
          </span>
        </div>
        <p className="text-xs text-secondary m-0 mt-1">{t("conversations.askInboxHint")}</p>
        {!canRespond && (
          <p className="text-xs text-warn m-0 mt-1">{t("conversations.askInboxRemoteBlocked")}</p>
        )}
        <div className="mt-2">{rowList}</div>
      </div>
    );
  }

  return (
    <div className="px-4 py-3 border-b border-outline-variant bg-surface-container-low shrink-0">
      {rowList}
    </div>
  );
}
