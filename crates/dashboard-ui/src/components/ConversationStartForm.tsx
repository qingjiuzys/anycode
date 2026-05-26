import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { api } from "@/api/client";
import type { WebChatResult } from "@/api/client/projects";
import type { SessionDetail } from "@/api/types";
import { useT } from "@/i18n/context";

export type ConversationStartSuccess = {
  session: SessionDetail;
  chat: WebChatResult;
};

type Props = {
  projectId: string;
  onSuccess?: (result: ConversationStartSuccess) => void;
  compact?: boolean;
};

export function ConversationStartForm({ projectId, onSuccess, compact }: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const titleTouched = useRef(false);

  const [sessionTitle, setSessionTitle] = useState("");
  const [prompt, setPrompt] = useState("");
  const [goal, setGoal] = useState("");
  const [kind, setKind] = useState<"run" | "goal">("run");

  const start = useMutation({
    mutationFn: () =>
      api.startConversation(projectId, {
        title: sessionTitle.trim() || undefined,
        prompt: prompt.trim(),
        kind,
        goal:
          kind === "goal"
            ? goal.trim() || prompt.trim()
            : undefined,
      }),
    onSuccess: (data) => {
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["project-triggers", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["running-sessions"] });
      onSuccess?.(data);
    },
  });

  function onPromptChange(value: string) {
    setPrompt(value);
    if (!titleTouched.current) {
      setSessionTitle(value.trim().slice(0, 120));
    }
  }

  function onTitleChange(value: string) {
    titleTouched.current = true;
    setSessionTitle(value);
  }

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    start.mutate();
  }

  const canSubmit = prompt.trim().length > 0 && !start.isPending;

  return (
    <form
      className={compact ? "space-y-3" : "space-y-4 max-w-2xl mx-auto"}
      onSubmit={onSubmit}
    >
      {!compact && (
        <div>
          <h3 className="text-base font-semibold m-0 mb-1">{t("conversations.startTitle")}</h3>
          <p className="text-sm text-secondary m-0">{t("conversations.startHint")}</p>
          <p className="text-xs text-secondary m-0 mt-1">{t("conversations.startNotice")}</p>
        </div>
      )}

      <div>
        <label className="block text-xs font-medium text-secondary mb-1">
          {t("conversations.sessionName")}
        </label>
        <input
          className="dw-input w-full"
          placeholder={t("conversations.sessionNamePlaceholder")}
          value={sessionTitle}
          onChange={(e) => onTitleChange(e.target.value)}
        />
        <p className="text-xs text-secondary mt-1 m-0">{t("conversations.sessionNameHint")}</p>
      </div>

      <div className="flex flex-wrap gap-2">
        <label className="flex items-center gap-1 text-sm">
          <input
            type="radio"
            name={`kind-${projectId}`}
            checked={kind === "run"}
            onChange={() => setKind("run")}
          />
          {t("projectDetail.triggerKindRun")}
        </label>
        <label className="flex items-center gap-1 text-sm">
          <input
            type="radio"
            name={`kind-${projectId}`}
            checked={kind === "goal"}
            onChange={() => setKind("goal")}
          />
          {t("projectDetail.triggerKindGoal")}
        </label>
      </div>

      {kind === "goal" && (
        <input
          className="dw-input w-full"
          placeholder={t("projectDetail.triggerGoalPlaceholder")}
          value={goal}
          onChange={(e) => setGoal(e.target.value)}
        />
      )}

      <div>
        <label className="block text-xs font-medium text-secondary mb-1">
          {t("conversations.taskPrompt")}
        </label>
        <textarea
          className="dw-input w-full min-h-[100px]"
          placeholder={t("projectDetail.triggerPromptPlaceholder")}
          value={prompt}
          onChange={(e) => onPromptChange(e.target.value)}
          required
        />
      </div>

      <button type="submit" className="dw-btn-primary" disabled={!canSubmit}>
        {start.isPending ? t("conversations.starting") : t("conversations.startTask")}
      </button>

      {start.isError && (
        <p className="text-sm text-error m-0">{(start.error as Error).message}</p>
      )}
    </form>
  );
}
