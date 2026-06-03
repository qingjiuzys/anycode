import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "@/api/client";
import type { WebChatResult } from "@/api/client/projects";
import type { SessionDetail, SessionWithProject } from "@/api/types";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export type ConversationStartSuccess = {
  session: SessionDetail;
  chat: WebChatResult;
};

type FollowUpProps = {
  mode: "follow-up";
  session: SessionWithProject;
  onSent?: (sessionId: string) => void;
};

type StartProps = {
  mode: "start";
  projectId: string;
  initialAgent?: string;
  compact?: boolean;
  onSuccess?: (result: ConversationStartSuccess) => void;
  onCancel?: () => void;
};

type Props = FollowUpProps | StartProps;

function parseSkillAllowlist(skillsJson: string): string[] | null {
  if (!skillsJson.trim()) return null;
  try {
    const v = JSON.parse(skillsJson) as { allowlist?: string[] };
    const list = v.allowlist?.filter(Boolean) ?? [];
    return list.length > 0 ? list : null;
  } catch {
    return null;
  }
}

export function ConversationComposer(props: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const titleTouched = useRef(false);

  const isStart = props.mode === "start";
  const session = props.mode === "follow-up" ? props.session : null;
  const projectId = props.mode === "start" ? props.projectId : session!.project_id;

  const [sessionTitle, setSessionTitle] = useState("");
  const [message, setMessage] = useState("");
  const [goal, setGoal] = useState("");
  const [kind, setKind] = useState<"run" | "goal">("run");
  const [agent, setAgent] = useState(
    props.mode === "start" ? (props.initialAgent ?? "") : session?.agent_type ?? "",
  );
  const [selectedSkills, setSelectedSkills] = useState<string[]>([]);
  const [skillsOpen, setSkillsOpen] = useState(false);

  useEffect(() => {
    if (props.mode === "start" && props.initialAgent) {
      setAgent(props.initialAgent);
    }
  }, [props]);

  const agentProfiles = useQuery({
    queryKey: ["agent-profiles"],
    queryFn: () => api.agentProfiles(),
  });

  const allSkills = useQuery({
    queryKey: ["skills", "picker"],
    queryFn: () => api.skills(100),
  });

  const skillOptions = useMemo(() => {
    const rows = allSkills.data?.skills ?? [];
    const profile = (agentProfiles.data?.profiles ?? []).find((p) => p.id === agent);
    const allow = profile ? parseSkillAllowlist(profile.skills_json) : null;
    const ids = rows.map((s) => s.id);
    if (!allow) return ids;
    return ids.filter((id) => allow.includes(id));
  }, [agent, agentProfiles.data?.profiles, allSkills.data?.skills]);

  useEffect(() => {
    setSelectedSkills((prev) => prev.filter((id) => skillOptions.includes(id)));
  }, [skillOptions]);

  const sendFollowUp = useMutation({
    mutationFn: (prompt: string) =>
      api.sendSessionMessage(session!.id, { prompt: prompt.trim() }),
    onSuccess: () => {
      setMessage("");
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["session", session!.id] });
      void queryClient.invalidateQueries({ queryKey: ["session-transcript", session!.id] });
      props.mode === "follow-up" && props.onSent?.(session!.id);
    },
  });

  const startSession = useMutation({
    mutationFn: () =>
      api.startConversation(projectId, {
        title: sessionTitle.trim() || undefined,
        prompt: message.trim(),
        kind,
        agent: agent.trim() || undefined,
        skills: selectedSkills.length > 0 ? selectedSkills : undefined,
        goal: kind === "goal" ? goal.trim() || message.trim() : undefined,
      }),
    onSuccess: (data) => {
      setMessage("");
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["session", data.session.id] });
      void queryClient.invalidateQueries({
        queryKey: ["session-transcript", data.session.id],
      });
      props.mode === "start" && props.onSuccess?.(data);
    },
  });

  const running = session?.status === "running";
  const pending = isStart ? startSession.isPending : sendFollowUp.isPending;
  const canSend =
    message.trim().length > 0 &&
    !pending &&
    (!isStart ? !running : true);

  function toggleSkill(id: string) {
    setSelectedSkills((prev) =>
      prev.includes(id) ? prev.filter((s) => s !== id) : [...prev, id],
    );
  }

  function onMessageChange(value: string) {
    setMessage(value);
    if (isStart && !titleTouched.current) {
      setSessionTitle(value.trim().slice(0, 120));
    }
  }

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSend) return;
    if (isStart) {
      startSession.mutate();
    } else {
      sendFollowUp.mutate(message.trim());
    }
  }

  const modelLabel =
    session?.model?.trim() ||
    session?.agent_type?.trim() ||
    t("conversations.agentDefault");

  const error = isStart ? startSession.error : sendFollowUp.error;

  return (
    <form className="dw-composer" onSubmit={onSubmit}>
      {isStart && !props.compact && (
        <div className="px-4 pt-3 pb-1 border-b border-outline-variant/50">
          <div className="flex flex-wrap items-center gap-3 mb-2">
            <input
              className="dw-input flex-1 min-w-[10rem] text-sm"
              placeholder={t("conversations.sessionNamePlaceholder")}
              value={sessionTitle}
              onChange={(e) => {
                titleTouched.current = true;
                setSessionTitle(e.target.value);
              }}
            />
            <div className="flex flex-wrap gap-2 text-sm">
              <label className="flex items-center gap-1">
                <input
                  type="radio"
                  name={`composer-kind-${projectId}`}
                  checked={kind === "run"}
                  onChange={() => setKind("run")}
                />
                {t("projectDetail.triggerKindRun")}
              </label>
              <label className="flex items-center gap-1">
                <input
                  type="radio"
                  name={`composer-kind-${projectId}`}
                  checked={kind === "goal"}
                  onChange={() => setKind("goal")}
                />
                {t("projectDetail.triggerKindGoal")}
              </label>
            </div>
          </div>
          {kind === "goal" && (
            <input
              className="dw-input w-full text-sm mb-2"
              placeholder={t("projectDetail.triggerGoalPlaceholder")}
              value={goal}
              onChange={(e) => setGoal(e.target.value)}
            />
          )}
        </div>
      )}

      <div className="dw-composer-input-wrap">
        {running && (
          <p className="text-xs text-secondary m-0 mb-2 flex items-center gap-2">
            <span className="inline-flex gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse [animation-delay:120ms]" />
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse [animation-delay:240ms]" />
            </span>
            {t("conversations.waitingForModel")}
          </p>
        )}
        <textarea
          className="dw-composer-textarea"
          placeholder={
            running
              ? t("conversations.composePlaceholderRunning")
              : isStart
                ? t("projectDetail.triggerPromptPlaceholder")
                : t("conversations.composePlaceholder")
          }
          value={message}
          onChange={(e) => onMessageChange(e.target.value)}
          disabled={running || pending}
          rows={isStart ? 4 : 3}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              if (canSend) {
                if (isStart) startSession.mutate();
                else sendFollowUp.mutate(message.trim());
              }
            }
          }}
        />
      </div>

      <div className="dw-composer-toolbar">
        <div className="flex flex-wrap items-center gap-2 min-w-0 flex-1">
          <label className="sr-only" htmlFor={`composer-agent-${projectId}`}>
            {t("conversations.agentPicker")}
          </label>
          <select
            id={`composer-agent-${projectId}`}
            className="dw-input text-xs py-1 max-w-[10rem]"
            value={agent}
            onChange={(e) => setAgent(e.target.value)}
            disabled={running || pending || !isStart}
            title={!isStart ? t("conversations.agentReadOnlyHint") : undefined}
          >
            <option value="">{t("conversations.agentDefault")}</option>
            {(agentProfiles.data?.profiles ?? []).map((p) => (
              <option key={p.id} value={p.id}>
                {p.id}
              </option>
            ))}
          </select>

          {skillOptions.length > 0 && (
            <div className="relative">
              <button
                type="button"
                className="dw-btn-secondary text-xs py-1"
                onClick={() => setSkillsOpen((v) => !v)}
                disabled={running || pending || !isStart}
                title={!isStart ? t("conversations.skillsReadOnlyHint") : undefined}
              >
                <Icon name="extension" size={14} />
                {t("conversations.skillsPicker")}
                {selectedSkills.length > 0 && (
                  <span className="ml-1 rounded-full bg-primary/15 text-primary px-1.5 text-[10px]">
                    {selectedSkills.length}
                  </span>
                )}
              </button>
              {skillsOpen && isStart && (
                <div className="absolute bottom-full left-0 mb-1 z-20 min-w-[14rem] max-w-[20rem] max-h-48 overflow-y-auto rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg p-2">
                  {skillOptions.map((id) => (
                    <label
                      key={id}
                      className="flex items-center gap-2 text-xs px-2 py-1 rounded hover:bg-surface-container-low cursor-pointer"
                    >
                      <input
                        type="checkbox"
                        checked={selectedSkills.includes(id)}
                        onChange={() => toggleSkill(id)}
                      />
                      <span className="font-code">{id}</span>
                    </label>
                  ))}
                </div>
              )}
            </div>
          )}

          <span
            className="text-[11px] text-secondary truncate max-w-[12rem]"
            title={modelLabel}
          >
            <Icon name="smart_toy" size={14} className="inline align-middle mr-1" />
            {modelLabel}
          </span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {isStart && props.onCancel && (
            <button type="button" className="dw-btn-ghost text-xs" onClick={props.onCancel}>
              {t("common.back")}
            </button>
          )}
          <button
            type="submit"
            className="dw-composer-send"
            disabled={!canSend}
            title={
              running
                ? t("conversations.composePlaceholderRunning")
                : isStart
                  ? t("conversations.startTask")
                  : t("conversations.composeSend")
            }
            aria-label={isStart ? t("conversations.startTask") : t("conversations.composeSend")}
          >
            {pending ? (
              <Icon name="hourglass_empty" size={20} />
            ) : running ? (
              <Icon name="pause" size={20} />
            ) : (
              <Icon name="arrow_upward" size={20} />
            )}
          </button>
        </div>
      </div>

      {error && (
        <p className="text-xs text-error m-0 px-4 pb-3">{(error as Error).message}</p>
      )}
    </form>
  );
}
