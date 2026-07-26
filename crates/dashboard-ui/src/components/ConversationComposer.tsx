import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { createPortal } from "react-dom";
import { api } from "@/api/client";
import type { WebChatResult } from "@/api/client/projects";
import type { SessionDetail, SessionWithProject } from "@/api/types";
import { FollowUpQueueCard } from "@/components/FollowUpQueueCard";
import { Icon } from "@/components/Icon";
import { ModelPicker } from "@/components/ModelPicker";
import { mergeVoiceTranscript, VoiceInputButton } from "@/components/VoiceInputButton";
import { appendOcrToMessage, ImageOcrButton } from "@/components/ImageOcrButton";
import { agentDisplayLabel, isPrimaryAgentId } from "@/lib/agentCatalog";
import { useLocale, useT } from "@/i18n/context";
import { skillDisplayDescription, skillDisplayName } from "@/lib/skillCatalog";
import { skillIconMeta, skillIconToneClass } from "@/lib/skillIcons";
import {
  mergeQueueItems,
  nextOptimisticSeq,
  removeOptimisticId,
  replaceOptimisticId,
  type OptimisticQueueItem,
} from "@/lib/optimisticMessageQueue";
import { useComposerIme } from "@/lib/composerIme";

type ConversationStartSuccess = {
  session: SessionDetail;
  chat: WebChatResult;
};

type FollowUpProps = {
  mode: "follow-up";
  session: SessionWithProject;
  onSent?: (sessionId: string) => void;
  hideWaitingIndicator?: boolean;
  onStreamingStart?: (sessionId: string) => void;
  onStreamingEnd?: () => void;
  waitingForQuestion?: boolean;
  turnActive?: boolean;
  chatStreamLive?: boolean;
  sseStatus?: "live" | "connecting" | "reconnecting" | "offline";
};

type StartProps = {
  mode: "start";
  projectId: string;
  initialAgent?: string;
  compact?: boolean;
  onSuccess?: (result: ConversationStartSuccess) => void;
  onCancel?: () => void;
  hideWaitingIndicator?: boolean;
  onStreamingStart?: (sessionId: string) => void;
};

type Props = FollowUpProps | StartProps;

type VisionAttachment = {
  mime_type: string;
  data_base64: string;
  previewUrl: string;
};

type TextAttachment = {
  filename: string;
  content: string;
};

const TEXT_FILE_ACCEPT = ".txt,.md,.json,.csv,.log,.pdf";
const ATTACH_ACCEPT = `image/*,${TEXT_FILE_ACCEPT}`;
const MAX_TEXT_FILE_BYTES = 1024 * 1024;
const MAX_IMAGE_BYTES = 4 * 1024 * 1024;

function isImageFile(file: File): boolean {
  if (file.type.startsWith("image/")) return true;
  return /\.(png|jpe?g|gif|webp|bmp|heic|heif)$/i.test(file.name);
}

const SLASH_COMMANDS = ["help", "skills"] as const;

/** Fixed popup above an anchor — avoids overflow:hidden clipping in composer. */
function useAnchoredAboveStyle(
  open: boolean,
  anchorRef: React.RefObject<HTMLElement | null>,
  opts?: { matchWidth?: boolean; minWidth?: number; maxWidth?: number },
) {
  const [style, setStyle] = useState<CSSProperties>({});
  const matchWidth = opts?.matchWidth ?? false;
  const minWidth = opts?.minWidth ?? 0;
  const maxWidth = opts?.maxWidth ?? 384;

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) return;
    const update = () => {
      const rect = anchorRef.current!.getBoundingClientRect();
      const width = matchWidth
        ? Math.min(rect.width, window.innerWidth - 16)
        : Math.min(Math.max(rect.width, minWidth), maxWidth, window.innerWidth - 16);
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
      setStyle({
        position: "fixed",
        left,
        bottom: window.innerHeight - rect.top + 8,
        width,
        zIndex: 300,
      });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open, anchorRef, matchWidth, minWidth, maxWidth]);

  return style;
}

async function fileToVisionAttachment(file: File): Promise<VisionAttachment> {
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]!);
  }
  return {
    mime_type: file.type || "image/jpeg",
    data_base64: btoa(binary),
    previewUrl: URL.createObjectURL(file),
  };
}

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

function parseMentionFilter(text: string): string | null {
  const match = text.match(/@([\w.-]*)$/);
  return match ? match[1] : null;
}

function parseSlashCommand(text: string): string | null {
  const trimmed = text.trimStart();
  if (!trimmed.startsWith("/") || trimmed.includes("\n")) return null;
  const body = trimmed.slice(1);
  if (!body || /^[\w.-]*$/.test(body)) {
    return body.toLowerCase();
  }
  return null;
}

/** Session-level approval delegation toggle ("托管模式"). */
function AutoApproveToggle({ sessionId }: { sessionId: string }) {
  const t = useT();
  const queryClient = useQueryClient();
  const state = useQuery({
    queryKey: ["session-auto-approve", sessionId],
    queryFn: () => api.sessionAutoApprove(sessionId),
    refetchInterval: 12_000,
  });
  const toggle = useMutation({
    mutationFn: (enabled: boolean) => api.setSessionAutoApprove(sessionId, enabled),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["session-auto-approve", sessionId] });
    },
  });
  const enabled = state.data?.enabled ?? false;
  return (
    <button
      type="button"
      className={`dw-voice-input-btn${enabled ? " dw-composer-icon-btn--danger" : ""}`}
      disabled={toggle.isPending}
      title={t("conversations.autoApproveHint")}
      aria-label={t("conversations.autoApprove")}
      aria-pressed={enabled}
      onClick={() => toggle.mutate(!enabled)}
    >
      <Icon name="verified_user" size={16} />
    </button>
  );
}

export function ConversationComposer(props: Props) {
  const t = useT();
  const queryClient = useQueryClient();
  const titleTouched = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const skillsTriggerRef = useRef<HTMLButtonElement>(null);

  const isStart = props.mode === "start";
  const session = props.mode === "follow-up" ? props.session : null;
  const projectId = props.mode === "start" ? props.projectId : session!.project_id;

  const [sessionTitle, setSessionTitle] = useState("");
  const [message, setMessage] = useState("");
  const [agent, setAgent] = useState(() => {
    if (props.mode === "start") return props.initialAgent ?? "";
    const fromSession = session?.agent_type ?? "";
    return fromSession === "general-purpose" ? "" : fromSession;
  });
  const [selectedSkills, setSelectedSkills] = useState<string[]>([]);
  const [skillsOpen, setSkillsOpen] = useState(false);
  const [slashOpen, setSlashOpen] = useState(false);
  const [mentionIndex, setMentionIndex] = useState(0);
  const [attachedImages, setAttachedImages] = useState<VisionAttachment[]>([]);
  const [attachedTextFiles, setAttachedTextFiles] = useState<TextAttachment[]>([]);
  const [attachmentError, setAttachmentError] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);
  const [optimisticQueue, setOptimisticQueue] = useState<OptimisticQueueItem[]>([]);
  const pendingOptimisticId = useRef<string | null>(null);
  const attachInputRef = useRef<HTMLInputElement>(null);
  const { compositionProps, shouldIgnoreEnterForIme } = useComposerIme();

  useEffect(() => {
    if (props.mode === "start" && props.initialAgent !== undefined) {
      setAgent(props.initialAgent === "general-purpose" ? "" : props.initialAgent);
    }
  }, [props]);

  useEffect(() => {
    if (props.mode === "follow-up" && session?.agent_type) {
      // Runtime may store Auto as general-purpose; keep picker on Auto display.
      setAgent(session.agent_type === "general-purpose" ? "" : session.agent_type);
    }
  }, [props.mode, session?.agent_type]);

  const agentProfiles = useQuery({
    queryKey: ["agent-profiles"],
    queryFn: () => api.agentProfiles(),
  });

  const allSkills = useQuery({
    queryKey: ["skills", "picker"],
    queryFn: () => api.skills(100),
  });

  const modelsRegistry = useQuery({
    queryKey: ["models-registry"],
    queryFn: () => api.getModelsRegistry(),
    staleTime: 60_000,
  });

  const chatSupportsVision = useMemo(() => {
    const activeId = modelsRegistry.data?.active?.chat;
    const item = (modelsRegistry.data?.items ?? []).find((m) => m.id === activeId);
    return (item?.capabilities ?? []).includes("vision");
  }, [modelsRegistry.data?.active?.chat, modelsRegistry.data?.items]);

  const locale = useLocale();

  const skillOptions = useMemo(() => {
    const rows = allSkills.data?.skills ?? [];
    const profile = (agentProfiles.data?.profiles ?? []).find((p) => p.id === agent);
    const allow = profile ? parseSkillAllowlist(profile.skills_json) : null;
    const ids = rows.map((s) => s.id);
    if (!allow) return ids;
    return ids.filter((id) => allow.includes(id));
  }, [agent, agentProfiles.data?.profiles, allSkills.data?.skills]);

  const skillById = useMemo(() => {
    const map = new Map<string, NonNullable<typeof allSkills.data>["skills"][number]>();
    for (const s of allSkills.data?.skills ?? []) {
      map.set(s.id, s);
    }
    return map;
  }, [allSkills.data?.skills]);

  const { primaryProfiles, moreProfiles } = useMemo(() => {
    const profiles = agentProfiles.data?.profiles ?? [];
    return {
      primaryProfiles: profiles.filter((p) => isPrimaryAgentId(p.id)),
      moreProfiles: profiles.filter((p) => !isPrimaryAgentId(p.id)),
    };
  }, [agentProfiles.data?.profiles]);

  useEffect(() => {
    setSelectedSkills((prev) => prev.filter((id) => skillOptions.includes(id)));
  }, [skillOptions]);

  const mentionFilter = parseMentionFilter(message);
  const mentionCandidates = useMemo(() => {
    if (mentionFilter === null) return [];
    const q = mentionFilter.toLowerCase();
    return skillOptions
      .filter((id) => id.toLowerCase().includes(q))
      .slice(0, 8);
  }, [mentionFilter, skillOptions]);

  const slashQuery = parseSlashCommand(message);
  const slashCandidates = useMemo(() => {
    if (slashQuery === null) return [];
    return SLASH_COMMANDS.filter(
      (cmd) => cmd.startsWith(slashQuery) || slashQuery === "",
    );
  }, [slashQuery]);

  useEffect(() => {
    setMentionIndex(0);
  }, [mentionFilter, slashQuery]);

  const running = session?.status === "running";
  const turnActive =
    props.mode === "follow-up"
      ? (props.turnActive ?? running)
      : false;
  const sseOffline =
    props.mode === "follow-up" && props.sseStatus != null && props.sseStatus !== "live";

  const sendFollowUp = useMutation({
    mutationFn: (payload: {
      prompt: string;
      agent?: string;
      skills?: string[];
      vision_images?: { mime_type: string; data_base64: string }[];
      text_files?: { filename: string; content: string }[];
      optimisticId?: string;
    }) => {
      const { optimisticId: _ignored, ...body } = payload;
      return api.sendSessionMessage(session!.id, body);
    },
    onMutate: (payload) => {
      if (!turnActive) return;
      const tempId = payload.optimisticId ?? `opt-${Date.now()}`;
      pendingOptimisticId.current = tempId;
      setOptimisticQueue((prev) => [
        ...prev,
        {
          id: tempId,
          prompt: payload.prompt.trim(),
          seq: nextOptimisticSeq(mergeQueueItems([], prev)),
        },
      ]);
    },
    onSuccess: (data) => {
      const tempId = pendingOptimisticId.current;
      pendingOptimisticId.current = null;
      setMessage("");
      attachedImages.forEach((img) => URL.revokeObjectURL(img.previewUrl));
      setAttachedImages([]);
      setAttachedTextFiles([]);
      if (data.queued && data.queue_id && tempId) {
        setOptimisticQueue((prev) =>
          replaceOptimisticId(prev, tempId, data.queue_id!, data.position ?? prev.length),
        );
      } else if (tempId) {
        setOptimisticQueue((prev) => removeOptimisticId(prev, tempId));
      }
      if (!data.queued) {
        props.mode === "follow-up" && props.onStreamingStart?.(session!.id);
      }
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["projects", "picker"] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["session", session!.id] });
      void queryClient.invalidateQueries({ queryKey: ["session-transcript", session!.id] });
      void queryClient.invalidateQueries({ queryKey: ["session-message-queue", session!.id] });
      props.mode === "follow-up" && props.onSent?.(session!.id);
    },
    onError: () => {
      const tempId = pendingOptimisticId.current;
      pendingOptimisticId.current = null;
      if (tempId) {
        setOptimisticQueue((prev) => removeOptimisticId(prev, tempId));
      }
    },
  });

  const refreshAfterCancel = useCallback(() => {
    props.mode === "follow-up" && props.onStreamingEnd?.();
    void queryClient.invalidateQueries({ queryKey: ["session", session!.id] });
    void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
    void queryClient.invalidateQueries({ queryKey: ["session-transcript", session!.id] });
    void queryClient.invalidateQueries({ queryKey: ["session-message-queue", session!.id] });
  }, [props, queryClient, session]);

  const cancelRun = useMutation({
    mutationFn: () => api.cancelSession(session!.id),
    onMutate: () => {
      setStopping(true);
      setOptimisticQueue([]);
    },
    onSuccess: refreshAfterCancel,
    onError: refreshAfterCancel,
    onSettled: () => {
      setStopping(false);
    },
  });

  const cancelQueued = useMutation({
    mutationFn: (queueId: string) => api.cancelQueuedMessage(session!.id, queueId),
    onMutate: (queueId) => {
      setOptimisticQueue((prev) => removeOptimisticId(prev, queueId));
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["session-message-queue", session!.id] });
    },
  });

  const startSession = useMutation({
    mutationFn: () =>
      api.startConversation(projectId, {
        title: sessionTitle.trim() || undefined,
        prompt: message.trim(),
        agent: agent.trim() || undefined,
        skills: selectedSkills.length > 0 ? selectedSkills : undefined,
        vision_images:
          attachedImages.length > 0
            ? attachedImages.map(({ mime_type, data_base64 }) => ({
                mime_type,
                data_base64,
              }))
            : undefined,
        text_files:
          attachedTextFiles.length > 0
            ? attachedTextFiles.map(({ filename, content }) => ({ filename, content }))
            : undefined,
        recycle_session: false,
      }),
    onSuccess: (data) => {
      setMessage("");
      attachedImages.forEach((img) => URL.revokeObjectURL(img.previewUrl));
      setAttachedImages([]);
      setAttachedTextFiles([]);
      props.mode === "start" && props.onStreamingStart?.(data.session.id);
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["projects", "picker"] });
      void queryClient.invalidateQueries({ queryKey: ["sessions", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["session", data.session.id] });
      void queryClient.invalidateQueries({
        queryKey: ["session-transcript", data.session.id],
      });
      props.mode === "start" && props.onSuccess?.(data);
    },
  });

  const waitingForQuestion =
    props.mode === "follow-up" ? Boolean(props.waitingForQuestion) : false;
  const hideWaiting =
    props.mode === "follow-up"
      ? Boolean(props.hideWaitingIndicator)
      : props.mode === "start"
        ? Boolean(props.hideWaitingIndicator)
        : false;
  const pending = isStart ? startSession.isPending : sendFollowUp.isPending;
  const messageQueue = useQuery({
    queryKey: ["session-message-queue", session?.id],
    queryFn: () => api.sessionMessageQueue(session!.id),
    enabled: !isStart && Boolean(session?.id),
    refetchInterval: turnActive && sseOffline ? 15_000 : false,
  });
  const queuedItems = mergeQueueItems(
    messageQueue.data?.items ?? [],
    optimisticQueue,
  );

  useEffect(() => {
    const serverItems = messageQueue.data?.items ?? [];
    if (serverItems.length === 0) return;
    setOptimisticQueue((prev) => prev.filter((item) => !item.id.startsWith("opt-")));
  }, [messageQueue.data?.items]);
  const hasContent =
    message.trim().length > 0 ||
    attachedImages.length > 0 ||
    attachedTextFiles.length > 0;
  const canSend =
    hasContent && !pending && !stopping && (!isStart ? !waitingForQuestion : true);
  const canStop = !isStart && turnActive && !pending && !stopping;

  const showMentionMenu = mentionFilter !== null;
  const showSlashMenu =
    slashCandidates.length > 0 && slashQuery !== null && message.trimStart().startsWith("/");
  const showSuggestMenu = showMentionMenu || (showSlashMenu && slashOpen);
  const suggestMenuStyle = useAnchoredAboveStyle(showSuggestMenu, textareaRef, {
    matchWidth: true,
  });
  const skillsMenuStyle = useAnchoredAboveStyle(skillsOpen, skillsTriggerRef, {
    minWidth: 288,
    maxWidth: 384,
  });

  function toggleSkill(id: string) {
    setSelectedSkills((prev) =>
      prev.includes(id) ? prev.filter((s) => s !== id) : [...prev, id],
    );
  }

  function applyMention(skillId: string) {
    setMessage((prev) => `${prev.replace(/@[\w.-]*$/, `@${skillId} `)}`);
    setSelectedSkills((prev) => (prev.includes(skillId) ? prev : [...prev, skillId]));
    setMentionIndex(0);
    textareaRef.current?.focus();
  }

  function applySlash(cmd: (typeof SLASH_COMMANDS)[number]) {
    if (cmd === "help") {
      setMessage(t("conversations.slashHelpText"));
      setSlashOpen(false);
      return;
    }
    if (cmd === "skills") {
      setMessage("");
      setSkillsOpen(true);
      setSlashOpen(false);
    }
  }

  function buildFollowUpPayload(prompt: string) {
    return {
      prompt: prompt.trim(),
      agent: agent.trim() || undefined,
      skills: selectedSkills.length > 0 ? selectedSkills : undefined,
      vision_images:
        attachedImages.length > 0
          ? attachedImages.map(({ mime_type, data_base64 }) => ({
              mime_type,
              data_base64,
            }))
          : undefined,
      text_files:
        attachedTextFiles.length > 0
          ? attachedTextFiles.map(({ filename, content }) => ({ filename, content }))
          : undefined,
    };
  }

  function submitMessage() {
    if (waitingForQuestion || stopping) return;
    if (!canSend) return;
    const payload = buildFollowUpPayload(message);
    if (isStart) {
      startSession.mutate();
    } else {
      const optimisticId = turnActive ? `opt-${Date.now()}` : undefined;
      sendFollowUp.mutate({ ...payload, optimisticId });
    }
  }

  function onMessageChange(value: string) {
    setMessage(value);
    if (isStart && !titleTouched.current) {
      setSessionTitle(value.trim().slice(0, 120));
    }
    setSlashOpen(value.trimStart().startsWith("/"));
  }

  function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    submitMessage();
  }

  const error = isStart ? startSession.error : sendFollowUp.error;

  function onComposerKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    const menu = showMentionMenu
      ? mentionCandidates
      : showSlashMenu && slashOpen
        ? slashCandidates
        : null;
    if (menu && menu.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setMentionIndex((i) => (i + 1) % menu.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setMentionIndex((i) => (i - 1 + menu.length) % menu.length);
        return;
      }
      if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey && menu.length > 0)) {
        if (e.key === "Enter" && shouldIgnoreEnterForIme(e)) return;
        e.preventDefault();
        if (showMentionMenu) {
          applyMention(mentionCandidates[mentionIndex]!);
        } else {
          applySlash(slashCandidates[mentionIndex]! as (typeof SLASH_COMMANDS)[number]);
        }
        return;
      }
    }
    if (e.key === "Escape" && (showMentionMenu || (showSlashMenu && slashOpen))) {
      e.preventDefault();
      if (showMentionMenu) {
        setMessage((prev) => prev.replace(/@[\w.-]*$/, ""));
      } else {
        setSlashOpen(false);
        setMessage((prev) => (prev.trimStart().startsWith("/") ? "" : prev));
      }
      return;
    }
    if (e.key === "Enter" && !e.shiftKey) {
      if (shouldIgnoreEnterForIme(e)) return;
      e.preventDefault();
      submitMessage();
    }
  }

  return (
    <div className="dw-composer-stack">
      {!isStart && queuedItems.length > 0 ? (
        <FollowUpQueueCard
          items={queuedItems}
          cancelling={cancelQueued.isPending}
          onCancel={(queueId) => cancelQueued.mutate(queueId)}
        />
      ) : null}
      <form className="dw-composer" onSubmit={onSubmit}>
      {isStart && !props.compact && (
        <div className="px-4 pt-3 pb-1">
          <input
            className="dw-input w-full text-sm"
            placeholder={t("conversations.sessionNamePlaceholder")}
            value={sessionTitle}
            onChange={(e) => {
              titleTouched.current = true;
              setSessionTitle(e.target.value);
            }}
          />
        </div>
      )}

      <div className="dw-composer-input-wrap relative">
        {running && !hideWaiting && !waitingForQuestion && (
          <p className="text-xs text-secondary m-0 mb-2 flex items-center gap-2">
            <span className="inline-flex gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse [animation-delay:120ms]" />
              <span className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse [animation-delay:240ms]" />
            </span>
            {stopping
              ? t("conversations.composeStopping")
              : t("conversations.thinkingWaiting")}
          </p>
        )}
        {waitingForQuestion && (
          <p className="text-xs text-primary m-0 mb-2 flex items-center gap-2">
            <Icon name="quiz" size={14} />
            {t("conversations.waitingForQuestion")}
          </p>
        )}
        {showSuggestMenu &&
          createPortal(
            <div
              className="rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg overflow-hidden"
              style={suggestMenuStyle}
              role="listbox"
            >
              {showMentionMenu && mentionCandidates.length === 0 ? (
                <p className="m-0 px-3 py-2.5 text-xs text-secondary">{t("conversations.mentionNoSkills")}</p>
              ) : null}
              {showMentionMenu &&
                mentionCandidates.map((id, idx) => (
                  <button
                    key={id}
                    type="button"
                    role="option"
                    aria-selected={idx === mentionIndex}
                    className={`w-full text-left px-3 py-2 text-xs font-code hover:bg-surface-container-low ${
                      idx === mentionIndex ? "bg-surface-container-low" : ""
                    }`}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      applyMention(id);
                    }}
                  >
                    @{id}
                  </button>
                ))}
              {showSlashMenu &&
                slashOpen &&
                slashCandidates.map((cmd, idx) => (
                  <button
                    key={cmd}
                    type="button"
                    role="option"
                    aria-selected={idx === mentionIndex}
                    className={`w-full text-left px-3 py-2 text-xs hover:bg-surface-container-low ${
                      idx === mentionIndex ? "bg-surface-container-low" : ""
                    }`}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      applySlash(cmd);
                    }}
                  >
                    /{cmd} — {t(`conversations.slashCmd.${cmd}`)}
                  </button>
                ))}
            </div>,
            document.body,
          )}
        <textarea
          ref={textareaRef}
          className="dw-composer-textarea"
          placeholder={
            stopping
              ? t("conversations.composeStopping")
              : turnActive
              ? t("conversations.composePlaceholderRunning")
              : isStart
                ? t("conversations.composePlaceholderStart")
                : t("conversations.composePlaceholder")
          }
          value={message}
          onChange={(e) => onMessageChange(e.target.value)}
          disabled={pending || stopping}
          rows={isStart ? 5 : 4}
          onKeyDown={onComposerKeyDown}
          {...compositionProps}
        />
        {(attachedTextFiles.length > 0 || attachedImages.length > 0) && (
          <div className="flex flex-wrap gap-2 mt-2 items-center">
            {attachedImages.map((img, idx) => (
              <div key={img.previewUrl} className="relative">
                <img
                  src={img.previewUrl}
                  alt=""
                  className="h-14 w-14 object-cover rounded-md border border-outline-variant"
                />
                <button
                  type="button"
                  className="absolute -top-1 -right-1 dw-btn-ghost text-[10px] px-1 py-0 min-h-0"
                  onClick={() => {
                    URL.revokeObjectURL(img.previewUrl);
                    setAttachedImages((prev) => prev.filter((_, i) => i !== idx));
                  }}
                >
                  ×
                </button>
              </div>
            ))}
            {attachedTextFiles.map((f, idx) => (
              <span
                key={`${f.filename}-${idx}`}
                className="inline-flex items-center gap-1 rounded-md border border-outline-variant bg-surface-container-low px-2 py-1 text-xs"
              >
                <Icon name="description" size={14} className="text-secondary" />
                <span className="font-code truncate max-w-[10rem]">{f.filename}</span>
                <button
                  type="button"
                  className="dw-btn-ghost text-[10px] px-1 py-0 min-h-0"
                  onClick={() =>
                    setAttachedTextFiles((prev) => prev.filter((_, i) => i !== idx))
                  }
                >
                  ×
                </button>
              </span>
            ))}
          </div>
        )}
        <input
          ref={attachInputRef}
          type="file"
          accept={ATTACH_ACCEPT}
          className="hidden"
          multiple
          onChange={async (e) => {
            const files = Array.from(e.target.files ?? []);
            e.target.value = "";
            const nextImages: VisionAttachment[] = [];
            const nextTexts: TextAttachment[] = [];
            for (const file of files) {
              if (isImageFile(file)) {
                if (!chatSupportsVision) {
                  setAttachmentError(t("conversations.attachmentVisionDisabled"));
                  continue;
                }
                if (attachedImages.length + nextImages.length >= 3) continue;
                if (file.size > MAX_IMAGE_BYTES) {
                  setAttachmentError(
                    t("conversations.attachmentImageTooLarge").replace("{name}", file.name),
                  );
                  continue;
                }
                nextImages.push(await fileToVisionAttachment(file));
                continue;
              }
              if (attachedTextFiles.length + nextTexts.length >= 3) continue;
              if (file.size > MAX_TEXT_FILE_BYTES) {
                setAttachmentError(
                  t("conversations.attachmentTextTooLarge").replace("{name}", file.name),
                );
                continue;
              }
              const lower = file.name.toLowerCase();
              if (lower.endsWith(".pdf")) {
                const buf = await file.arrayBuffer();
                const bytes = new Uint8Array(buf);
                let binary = "";
                for (let i = 0; i < bytes.length; i += 1) {
                  binary += String.fromCharCode(bytes[i]!);
                }
                nextTexts.push({ filename: file.name, content: btoa(binary) });
              } else {
                const content = await file.text();
                nextTexts.push({ filename: file.name, content });
              }
            }
            if (nextImages.length > 0 || nextTexts.length > 0) setAttachmentError(null);
            if (nextImages.length > 0) {
              setAttachedImages((prev) => [...prev, ...nextImages].slice(0, 3));
            }
            if (nextTexts.length > 0) {
              setAttachedTextFiles((prev) => [...prev, ...nextTexts].slice(0, 3));
            }
          }}
        />
        {attachmentError && (
          <p className="text-xs text-error m-0 mt-2">{attachmentError}</p>
        )}
      </div>

      <div className="dw-composer-toolbar">
        <div className="flex flex-wrap items-center gap-2 min-w-0 flex-1">
          <label className="sr-only" htmlFor={`composer-agent-${projectId}`}>
            {t("conversations.agentPicker")}
          </label>
          <select
            id={`composer-agent-${projectId}`}
            className="dw-composer-chip dw-composer-chip--select"
            value={agent}
            onChange={(e) => setAgent(e.target.value)}
            disabled={running || pending}
            title={
              agent
                ? `${agentDisplayLabel(agent, t)} (${agent}) · ${t("conversations.agentFollowUpHint")}`
                : `${t("conversations.agentAuto")} · ${t("conversations.agentAutoSubtitle")}`
            }
          >
            <option value="">{t("conversations.agentAutoLabel")}</option>
            {primaryProfiles.length > 0 && (
              <optgroup label={t("conversations.agentGroupPrimary")}>
                {primaryProfiles.map((p) => (
                  <option key={p.id} value={p.id}>
                    {agentDisplayLabel(p.id, t)}
                  </option>
                ))}
              </optgroup>
            )}
            {moreProfiles.length > 0 && (
              <optgroup label={t("conversations.agentGroupMore")}>
                {moreProfiles.map((p) => (
                  <option key={p.id} value={p.id}>
                    {agentDisplayLabel(p.id, t)}
                  </option>
                ))}
              </optgroup>
            )}
          </select>

          <ModelPicker disabled={running || pending} compact />

          {skillOptions.length > 0 && (
            <div className="relative">
              <button
                ref={skillsTriggerRef}
                type="button"
                className="dw-composer-chip"
                onClick={() => setSkillsOpen((v) => !v)}
                disabled={running || pending}
                aria-expanded={skillsOpen}
              >
                <Icon name="extension" size={16} />
                {t("conversations.skillsPicker")}
                {selectedSkills.length > 0 && (
                  <span className="dw-composer-chip__badge">
                    {selectedSkills.length}
                  </span>
                )}
              </button>
              {skillsOpen &&
                createPortal(
                  <>
                    <button
                      type="button"
                      className="fixed inset-0 z-[299] cursor-default border-0 bg-transparent"
                      aria-hidden
                      onClick={() => setSkillsOpen(false)}
                    />
                    <div
                      className="dw-composer-skills-menu"
                      style={skillsMenuStyle}
                      role="menu"
                    >
                      {skillOptions.map((id) => {
                        const row = skillById.get(id);
                        const desc = row ? skillDisplayDescription(row, locale) : "";
                        const label = row ? skillDisplayName(row, locale) : id;
                        const active = selectedSkills.includes(id);
                        const { icon, tone } = skillIconMeta(row ?? { id });
                        return (
                          <button
                            key={id}
                            type="button"
                            role="menuitemcheckbox"
                            aria-checked={active}
                            className={`dw-composer-skills-menu__item${active ? " is-active" : ""}`}
                            onClick={() => toggleSkill(id)}
                          >
                            <span
                              className={`dw-composer-skills-menu__icon ${skillIconToneClass(tone)}`}
                            >
                              <Icon name={icon} size={16} />
                            </span>
                            <span className="min-w-0 flex-1 text-left">
                              <span className="text-sm font-medium block truncate">{label}</span>
                              {desc ? (
                                <span className="text-[13px] leading-snug text-secondary line-clamp-2 block mt-0.5">
                                  {desc}
                                </span>
                              ) : null}
                            </span>
                            {active ? (
                              <Icon name="check" size={18} className="text-primary shrink-0" />
                            ) : null}
                          </button>
                        );
                      })}
                    </div>
                  </>,
                  document.body,
                )}
            </div>
          )}

          <button
            type="button"
            className="dw-voice-input-btn"
            disabled={
              running ||
              pending ||
              (attachedImages.length >= 3 && attachedTextFiles.length >= 3)
            }
            title={
              chatSupportsVision
                ? t("conversations.attachFile")
                : t("conversations.attachmentVisionDisabled")
            }
            aria-label={t("conversations.attachFile")}
            onClick={() => attachInputRef.current?.click()}
          >
            <Icon name="attach_file" size={16} />
          </button>

          <ImageOcrButton
            disabled={running || pending}
            images={attachedImages.map(({ mime_type, data_base64 }) => ({ mime_type, data_base64 }))}
            onText={(text) => setMessage((prev) => appendOcrToMessage(prev, text))}
          />

          {session && <AutoApproveToggle sessionId={session.id} />}
        </div>

        <div className="dw-composer-toolbar__actions">
          {isStart && props.onCancel && (
            <button type="button" className="dw-btn-ghost text-xs" onClick={props.onCancel}>
              {t("common.back")}
            </button>
          )}
          {canStop && (
            <button
              type="button"
              className="dw-composer-stop"
              disabled={cancelRun.isPending}
              title={t("conversations.composeStop")}
              aria-label={t("conversations.composeStop")}
              onClick={() => cancelRun.mutate()}
            >
              {cancelRun.isPending || stopping ? (
                <Icon name="hourglass_empty" size={18} />
              ) : (
                <Icon name="stop" size={18} />
              )}
            </button>
          )}
          <VoiceInputButton
            disabled={running || pending}
            onTranscribed={(text) => setMessage((prev) => mergeVoiceTranscript(prev, text))}
          />
          <button
            type="submit"
            className="dw-composer-send"
            disabled={!canSend}
            title={
              turnActive
                ? t("conversations.messageQueuedHint")
                : isStart
                  ? t("conversations.startTask")
                  : t("conversations.composeSend")
            }
            aria-label={isStart ? t("conversations.startTask") : t("conversations.composeSend")}
          >
            {pending ? (
              <Icon name="hourglass_empty" size={20} />
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
    </div>
  );
}
