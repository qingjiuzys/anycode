import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import type { PendingQuestionsResponse } from "@/api/types";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  sessionId: string;
};

export function AskUserQuestionInbox({ sessionId }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const queryKey = ["pending-questions", sessionId] as const;
  const [otherText, setOtherText] = useState<Record<string, string>>({});
  const [selected, setSelected] = useState<Record<string, Set<string>>>({});

  const inbox = useQuery({
    queryKey,
    queryFn: () => api.pendingQuestions({ limit: 5, sessionId }),
    staleTime: 3_000,
    refetchInterval: 10_000,
    refetchIntervalInBackground: false,
  });

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
    onMutate: async ({ questionId }) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData<PendingQuestionsResponse>(queryKey);
      if (previous) {
        queryClient.setQueryData<PendingQuestionsResponse>(queryKey, {
          ...previous,
          pending: previous.pending.filter((row) => row.question_id !== questionId),
        });
      }
      return { previous };
    },
    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKey, context.previous);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey });
    },
  });

  const rows = inbox.data?.pending ?? [];
  const canRespond = inbox.data?.respond_allowed ?? false;

  if (rows.length === 0) return null;

  return (
    <div className="px-4 py-3 border-b border-outline-variant bg-surface-container-low shrink-0 space-y-3">
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
            className="rounded-lg border border-primary/30 bg-surface-container-lowest p-3"
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
}
