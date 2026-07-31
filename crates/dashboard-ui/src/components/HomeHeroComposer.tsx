import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { createPortal } from "react-dom";
import { buildConversationsHref, conversationSearchParams } from "@/lib/conversationsSearch";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { ProjectPicker } from "@/components/ProjectPicker";
import { ModelPicker } from "@/components/ModelPicker";
import { mergeVoiceTranscript, VoiceInputButton } from "@/components/VoiceInputButton";
import { useLocale, useT } from "@/i18n/context";
import { useComposerIme } from "@/lib/composerIme";
import { parseComposerSlashInput, parseSlashQuery } from "@/lib/composerSlash";
import {
  composerModeForSend,
  grillSlashCommand,
  isGrillSlashToken,
  loadGrillMode,
  saveGrillMode,
  shouldExitGrillMode,
} from "@/lib/grillMode";
import {
  GOAL_AGENT_ID,
  goalSlashCommand,
  isGoalSlashToken,
  loadGoalMode,
  saveGoalMode,
} from "@/lib/goalMode";
import { useAnchoredAboveStyle } from "@/lib/useAnchoredAboveStyle";

type Sse = "live" | "connecting" | "reconnecting" | "offline";

const DISMISS_BROWSER_KEY = "anycode-home-browser-hint-dismiss";

export function HomeHeroComposer({
  sseStatus,
  projectOptions,
  projectId,
  onProjectChange,
  initialProjectId,
  blockedCount = 0,
  pendingCount = 0,
  budgetExceededCount = 0,
  prompt: promptProp,
  onPromptChange,
  onSessionStarted,
  onSelectDirectory,
}: {
  sseStatus: Sse;
  projectOptions: { id: string; name: string }[];
  projectId?: string;
  onProjectChange?: (projectId: string) => void;
  initialProjectId?: string;
  blockedCount?: number;
  pendingCount?: number;
  budgetExceededCount?: number;
  prompt?: string;
  onPromptChange?: (value: string) => void;
  onSessionStarted?: (data: {
    session: import("@/api/types").SessionDetail;
    projectId: string;
    projectName: string;
  }) => void;
  onSelectDirectory?: () => void;
}) {
  const t = useT();
  const locale = useLocale();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [internalPrompt, setInternalPrompt] = useState("");
  const prompt = promptProp ?? internalPrompt;
  const setPrompt = onPromptChange ?? setInternalPrompt;
  const [internalProjectId, setInternalProjectId] = useState("");
  const isControlled = projectId !== undefined;
  const resolvedProjectId = isControlled ? projectId : internalProjectId;
  const setResolvedProjectId = (nextProjectId: string) => {
    if (onProjectChange) {
      onProjectChange(nextProjectId);
      return;
    }
    setInternalProjectId(nextProjectId);
  };
  const [browserHintDismissed, setBrowserHintDismissed] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { compositionProps, shouldIgnoreEnterForIme } = useComposerIme();

  const modeStorageKey = resolvedProjectId ? `project:${resolvedProjectId}` : undefined;
  const [grillMode, setGrillMode] = useState(() => loadGrillMode(modeStorageKey));
  const [goalMode, setGoalMode] = useState(() => loadGoalMode(modeStorageKey));
  const [slashOpen, setSlashOpen] = useState(false);
  const [slashIndex, setSlashIndex] = useState(0);

  const slashCommands = useMemo(
    () => [grillSlashCommand(locale), goalSlashCommand(locale)],
    [locale],
  );

  useEffect(() => {
    const grill = loadGrillMode(modeStorageKey);
    const goal = loadGoalMode(modeStorageKey);
    setGrillMode(grill);
    setGoalMode(goal && !grill);
  }, [modeStorageKey]);

  useEffect(() => {
    saveGrillMode(modeStorageKey, grillMode);
  }, [modeStorageKey, grillMode]);

  useEffect(() => {
    saveGoalMode(modeStorageKey, goalMode);
  }, [modeStorageKey, goalMode]);

  const browser = useQuery({
    queryKey: ["browser-connector"],
    queryFn: api.browserConnector,
  });

  const enableBrowser = useMutation({
    mutationFn: () => api.setBrowserConnector(true),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["browser-connector"] });
      void queryClient.invalidateQueries({ queryKey: ["doctor"] });
    },
  });

  useEffect(() => {
    setBrowserHintDismissed(sessionStorage.getItem(DISMISS_BROWSER_KEY) === "1");
  }, []);

  const seededFocusDone = useRef(false);
  useEffect(() => {
    if (seededFocusDone.current) return;
    if (!prompt.trim()) return;
    seededFocusDone.current = true;
    requestAnimationFrame(() => {
      const el = textareaRef.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(prompt.length, prompt.length);
    });
  }, [prompt]);

  useEffect(() => {
    if (isControlled) return;
    if (
      initialProjectId &&
      projectOptions.some((p) => p.id === initialProjectId) &&
      internalProjectId !== initialProjectId
    ) {
      setInternalProjectId(initialProjectId);
      return;
    }
    if (!internalProjectId && projectOptions.length > 0) {
      setInternalProjectId(projectOptions[0]!.id);
    }
  }, [initialProjectId, internalProjectId, isControlled, projectOptions]);

  const start = useMutation({
    mutationFn: (vars: { prompt: string; grill: boolean; goal: boolean }) =>
      api.startConversation(resolvedProjectId, {
        prompt: vars.prompt.trim(),
        agent: vars.goal ? GOAL_AGENT_ID : undefined,
        composer_mode: composerModeForSend(vars.grill),
        recycle_session: false,
      }),
    onSuccess: (data, vars) => {
      if (vars.grill) {
        saveGrillMode(data.session.id, true);
        saveGrillMode(modeStorageKey, false);
      }
      if (vars.goal) {
        saveGoalMode(data.session.id, true);
        saveGoalMode(modeStorageKey, false);
      }
      setPrompt("");
      setGrillMode(false);
      setGoalMode(false);
      const projectName =
        projectOptions.find((p) => p.id === resolvedProjectId)?.name ?? "";
      onSessionStarted?.({
        session: data.session,
        projectId: resolvedProjectId,
        projectName,
      });
      void queryClient.invalidateQueries({ queryKey: ["all-sessions"] });
      void queryClient.invalidateQueries({ queryKey: ["all-sessions", "sidebar"] });
      void queryClient.invalidateQueries({ queryKey: ["projects", "picker"] });
      void queryClient.invalidateQueries({ queryKey: ["session", data.session.id] });
      void queryClient.invalidateQueries({
        queryKey: ["session-transcript", data.session.id],
      });
      const canon = conversationSearchParams({
        session: data.session.id,
        project: resolvedProjectId,
      });
      const href = buildConversationsHref(canon);
      window.history.replaceState(window.history.state, "", href);
      void navigate({
        to: "/conversations",
        search: () => canon,
      });
    },
  });

  const connected = sseStatus === "live";
  const showBrowserRow =
    !browserHintDismissed &&
    browser.data?.bundled === true &&
    browser.data.enabled !== true;

  function dismissBrowserHint() {
    sessionStorage.setItem(DISMISS_BROWSER_KEY, "1");
    setBrowserHintDismissed(true);
  }

  function enableGoalMode() {
    setGrillMode(false);
    setGoalMode(true);
  }

  function disableGoalMode() {
    setGoalMode(false);
  }

  function enableGrillMode() {
    disableGoalMode();
    setGrillMode(true);
  }

  function slashCmdLabel(cmd: string): string {
    if (isGrillSlashToken(cmd)) return t("conversations.slashCmd.grill");
    if (isGoalSlashToken(cmd)) return t("conversations.slashCmd.goal");
    return cmd;
  }

  function applySlash(cmd: string) {
    const parsed = parseComposerSlashInput(prompt);
    if (isGrillSlashToken(cmd)) {
      if (grillMode && parsed.bareSlash) {
        setGrillMode(false);
      } else {
        enableGrillMode();
      }
      setPrompt(parsed.mode === "grill" ? parsed.prompt : "");
      setSlashOpen(false);
      textareaRef.current?.focus();
      return;
    }
    if (isGoalSlashToken(cmd)) {
      if (goalMode && parsed.bareSlash) {
        disableGoalMode();
      } else {
        enableGoalMode();
      }
      setPrompt(parsed.mode === "goal" ? parsed.prompt : "");
      setSlashOpen(false);
      textareaRef.current?.focus();
    }
  }

  function submitMessage() {
    const parsed = parseComposerSlashInput(prompt);
    const grillActive = grillMode || parsed.mode === "grill";
    const goalActive = goalMode || parsed.mode === "goal";

    if (parsed.bareSlash && parsed.mode) {
      if (parsed.mode === "grill") {
        if (grillMode) setGrillMode(false);
        else enableGrillMode();
      } else if (goalMode) {
        disableGoalMode();
      } else {
        enableGoalMode();
      }
      setPrompt("");
      setSlashOpen(false);
      return;
    }

    const outgoingPrompt = parsed.mode ? parsed.prompt : prompt.trim();
    if (!outgoingPrompt || !resolvedProjectId || start.isPending) return;

    if (parsed.mode === "grill" && !grillMode) enableGrillMode();
    if (parsed.mode === "goal" && !goalMode) enableGoalMode();

    start.mutate({
      prompt: outgoingPrompt,
      grill: grillActive,
      goal: goalActive,
    });

    if (grillActive && shouldExitGrillMode(outgoingPrompt)) {
      setGrillMode(false);
    }
  }

  function onPromptInput(value: string) {
    setPrompt(value);
    setSlashOpen(value.trimStart().startsWith("/"));
  }

  const slashQuery = parseSlashQuery(prompt);
  const slashCandidates = useMemo(() => {
    if (slashQuery === null) return [];
    return slashCommands.filter((cmd) => cmd.startsWith(slashQuery) || slashQuery === "");
  }, [slashQuery, slashCommands]);

  useEffect(() => {
    setSlashIndex(0);
  }, [slashQuery]);

  const showSlashMenu =
    slashOpen && slashCandidates.length > 0 && slashQuery !== null && prompt.trimStart().startsWith("/");
  const slashMenuStyle = useAnchoredAboveStyle(showSlashMenu, textareaRef, {
    matchWidth: true,
  });

  const parsedForSubmit = parseComposerSlashInput(prompt);
  const canToggleMode = Boolean(parsedForSubmit.bareSlash && parsedForSubmit.mode);
  const outgoingForSubmit = parsedForSubmit.mode
    ? parsedForSubmit.prompt
    : prompt.trim();
  const canSubmit =
    !start.isPending &&
    (canToggleMode || (outgoingForSubmit.length > 0 && resolvedProjectId.length > 0));
  const hasAlerts = blockedCount > 0 || pendingCount > 0 || budgetExceededCount > 0;

  const statusLabel = connected
    ? t("home.hero.statusLive")
    : sseStatus === "connecting" || sseStatus === "reconnecting"
      ? t("home.hero.statusConnecting")
      : t("home.hero.statusOffline");

  const placeholder = grillMode
    ? t("conversations.grillModePlaceholder")
    : goalMode
      ? t("conversations.goalModePlaceholder")
      : t("home.hero.placeholder");

  function onComposerKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (showSlashMenu && slashCandidates.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSlashIndex((i) => (i + 1) % slashCandidates.length);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setSlashIndex((i) => (i - 1 + slashCandidates.length) % slashCandidates.length);
        return;
      }
      if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey)) {
        if (e.key === "Enter" && shouldIgnoreEnterForIme(e)) return;
        e.preventDefault();
        const parsed = parseComposerSlashInput(prompt);
        if (e.key === "Enter" && parsed.mode && !parsed.bareSlash) {
          submitMessage();
        } else {
          applySlash(slashCandidates[slashIndex]!);
        }
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setSlashOpen(false);
        if (prompt.trimStart().startsWith("/")) setPrompt("");
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey && canSubmit) {
      if (shouldIgnoreEnterForIme(e)) return;
      e.preventDefault();
      submitMessage();
    }
  }

  return (
    <div className="dw-hero-composer">
      <div className="dw-hero-composer__card glass-panel">
        {(grillMode || goalMode) && (
          <div className="dw-hero-composer__modes">
            {grillMode ? (
              <div className="flex items-center gap-2">
                <span className="inline-flex items-center gap-1.5 rounded-full border border-primary/30 bg-primary/8 px-2.5 py-1 text-xs text-primary">
                  <Icon name="quiz" size={14} />
                  {t("conversations.grillModeActive")}
                </span>
                <button
                  type="button"
                  className="dw-btn-ghost text-xs py-0.5 px-1.5"
                  onClick={() => setGrillMode(false)}
                >
                  {t("conversations.grillModeExit")}
                </button>
              </div>
            ) : null}
            {goalMode ? (
              <div className="flex items-center gap-2">
                <span className="inline-flex items-center gap-1.5 rounded-full border border-secondary/30 bg-secondary/8 px-2.5 py-1 text-xs text-secondary">
                  <Icon name="flag" size={14} />
                  {t("conversations.goalModeActive")}
                </span>
                <button
                  type="button"
                  className="dw-btn-ghost text-xs py-0.5 px-1.5"
                  onClick={() => disableGoalMode()}
                >
                  {t("conversations.goalModeExit")}
                </button>
              </div>
            ) : null}
          </div>
        )}
        <div className="dw-hero-composer__input-wrap relative">
          {showSlashMenu &&
            createPortal(
              <div
                className="rounded-lg border border-outline-variant bg-surface-container-lowest shadow-lg overflow-hidden"
                style={slashMenuStyle}
                role="listbox"
              >
                {slashCandidates.map((cmd, idx) => (
                  <button
                    key={cmd}
                    type="button"
                    role="option"
                    aria-selected={idx === slashIndex}
                    className={`w-full text-left px-3 py-2 text-xs hover:bg-surface-container-low ${
                      idx === slashIndex ? "bg-surface-container-low" : ""
                    }`}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      applySlash(cmd);
                    }}
                  >
                    /{cmd} — {slashCmdLabel(cmd)}
                  </button>
                ))}
              </div>,
              document.body,
            )}
          <textarea
            ref={textareaRef}
            className="dw-hero-composer__textarea"
            placeholder={placeholder}
            value={prompt}
            rows={7}
            onChange={(e) => onPromptInput(e.target.value)}
            onKeyDown={onComposerKeyDown}
            {...compositionProps}
          />
        </div>
        <div className="dw-hero-composer__toolbar">
          <ProjectPicker
            value={resolvedProjectId}
            onChange={setResolvedProjectId}
            options={projectOptions}
            disabled={start.isPending}
            onSelectDirectory={onSelectDirectory}
          />
          <ModelPicker disabled={start.isPending} />
          <div className="dw-hero-composer__toolbar-actions">
            <VoiceInputButton
              disabled={start.isPending}
              onTranscribed={(text) => onPromptInput(mergeVoiceTranscript(prompt, text))}
            />
            <button
              type="button"
              className="dw-hero-composer__submit"
              disabled={!canSubmit}
              aria-label={t("home.hero.send")}
              onClick={() => submitMessage()}
            >
              <Icon name="arrow_upward" size={20} />
            </button>
          </div>
        </div>
      </div>

      <div className="dw-hero-composer__meta">
        <span
          className={`dw-hero-composer__status-dot ${connected ? "dw-hero-composer__status-dot--ok" : "dw-hero-composer__status-dot--warn"}`}
          aria-hidden
        />
        <span className={connected ? "text-secondary" : "text-error"}>{statusLabel}</span>
      </div>

      {showBrowserRow && (
        <div className="dw-hero-composer__browser-hint">
          <span className="text-xs text-secondary">{t("home.hero.browserHint")}</span>
          <div className="flex items-center gap-2 shrink-0">
            <button
              type="button"
              className="dw-hero-composer__hint-btn"
              disabled={enableBrowser.isPending}
              onClick={() => enableBrowser.mutate()}
            >
              {t("home.hero.browserEnable")}
            </button>
            <button type="button" className="dw-hero-composer__hint-btn" onClick={dismissBrowserHint}>
              {t("home.hero.browserDismiss")}
            </button>
          </div>
        </div>
      )}

      {hasAlerts && (
        <div className="dw-hero-composer__alerts">
          {blockedCount > 0 && (
            <Link
              to={buildConversationsHref({ filter: "blocked" })}
              className="dw-hero-composer__alert-chip dw-hero-composer__alert-chip--error"
            >
              {t("home.hero.alertBlocked").replace("{n}", String(blockedCount))}
            </Link>
          )}
          {pendingCount > 0 && (
            <Link
              to={buildConversationsHref({ filter: "needs_approval" })}
              className="dw-hero-composer__alert-chip dw-hero-composer__alert-chip--warn"
            >
              {t("home.hero.alertPending").replace("{n}", String(pendingCount))}
            </Link>
          )}
          {budgetExceededCount > 0 && (
            <Link
              to={buildConversationsHref({ filter: "budget" })}
              className="dw-hero-composer__alert-chip dw-hero-composer__alert-chip--warn"
            >
              {t("home.hero.alertBudget").replace("{n}", String(budgetExceededCount))}
            </Link>
          )}
        </div>
      )}

      {start.isError && (
        <p className="text-xs text-error m-0 text-center">
          {(start.error as Error).message || t("home.hero.startError")}
        </p>
      )}
    </div>
  );
}
