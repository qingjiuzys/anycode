import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import type { SessionWithProject } from "@/api/types";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

type Props = {
  session: SessionWithProject;
  onSent?: (sessionId: string) => void;
};

export function ConversationCompose({ session, onSent }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const [message, setMessage] = useState("");

  const send = useMutation({
    mutationFn: (prompt: string) =>
      api.sendSessionMessage(session.id, {
        prompt: prompt.trim(),
      }),
    onSuccess: () => {
      setMessage("");
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", session.project_id] });
      void queryClient.invalidateQueries({ queryKey: ["project-triggers", session.project_id] });
      void queryClient.invalidateQueries({ queryKey: ["running-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["session", session.id] });
      void queryClient.invalidateQueries({ queryKey: ["session-transcript", session.id] });
      onSent?.(session.id);
    },
  });

  const running = session.status === "running";
  const canSend = message.trim().length > 0 && !send.isPending && !running;

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSend) return;
    send.mutate(message.trim());
  }

  return (
    <div className="space-y-2">
      {running ? (
        <p className="text-xs text-secondary m-0">{t("conversations.composeRunningHint")}</p>
      ) : (
        <p className="text-xs text-secondary m-0">{t("conversations.composeHint")}</p>
      )}
      <form className="flex gap-2 items-end" onSubmit={onSubmit}>
        <textarea
          className="dw-input flex-1 min-h-[44px] max-h-32 resize-y text-sm"
          placeholder={
            running
              ? t("conversations.composePlaceholderRunning")
              : t("conversations.composePlaceholder")
          }
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          disabled={running || send.isPending}
          rows={2}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              if (canSend) send.mutate(message.trim());
            }
          }}
        />
        <button
          type="submit"
          className="dw-btn-primary shrink-0 h-[44px] px-4"
          disabled={!canSend}
          title={t("conversations.composeSend")}
        >
          {send.isPending ? (
            t("conversations.starting")
          ) : (
            <span className="inline-flex items-center gap-1">
              <Icon name="send" size={16} />
              {t("conversations.composeSend")}
            </span>
          )}
        </button>
      </form>
      {send.isError && (
        <p className="text-xs text-error m-0">{(send.error as Error).message}</p>
      )}
    </div>
  );
}
