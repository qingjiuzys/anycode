import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { buildConversationsHref, conversationSearchParams } from "@/lib/conversationsSearch";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { ProjectPicker } from "@/components/ProjectPicker";
import { ModelPicker } from "@/components/ModelPicker";
import { mergeVoiceTranscript, VoiceInputButton } from "@/components/VoiceInputButton";
import { useT, useLocale } from "@/i18n/context";
import { SCENARIO_CARDS, scenarioPrompt, type ScenarioCard } from "@/lib/scenarioCards";

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
    mutationFn: (opts?: { agent?: string; skills?: string[]; prompt?: string }) =>
      api.startConversation(resolvedProjectId, {
        prompt: (opts?.prompt ?? prompt).trim(),
        agent: opts?.agent,
        skills: opts?.skills,
        recycle_session: false,
      }),
    onSuccess: (data) => {
      setPrompt("");
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

  const canSubmit =
    prompt.trim().length > 0 && resolvedProjectId.length > 0 && !start.isPending;
  const hasAlerts = blockedCount > 0 || pendingCount > 0 || budgetExceededCount > 0;

  const statusLabel = connected
    ? t("home.hero.statusLive")
    : sseStatus === "connecting" || sseStatus === "reconnecting"
      ? t("home.hero.statusConnecting")
      : t("home.hero.statusOffline");

  function applyScenario(card: ScenarioCard) {
    const text = scenarioPrompt(card, locale);
    setPrompt(text);
    requestAnimationFrame(() => {
      const el = textareaRef.current;
      if (!el) return;
      el.focus();
      el.setSelectionRange(text.length, text.length);
    });
  }

  return (
    <div className="dw-hero-composer">
      <div className="dw-hero-composer__scenarios">
        <span className="dw-hero-composer__scenarios-label">{t("home.hero.scenariosLabel")}</span>
        <div className="dw-hero-composer__scenario-row">
          {SCENARIO_CARDS.map((card) => (
            <button
              key={card.id}
              type="button"
              className="dw-hero-composer__scenario-chip"
              onClick={() => applyScenario(card)}
            >
              <Icon name={card.icon} size={16} />
              <span>{t(`home.scenarios.${card.id}`)}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="dw-hero-composer__card glass-panel">
        <textarea
          ref={textareaRef}
          className="dw-hero-composer__textarea"
          placeholder={t("home.hero.placeholder")}
          value={prompt}
          rows={7}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && canSubmit) {
              e.preventDefault();
              start.mutate({});
            }
          }}
        />
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
              onTranscribed={(text) => setPrompt(mergeVoiceTranscript(prompt, text))}
            />
            <button
              type="button"
              className="dw-hero-composer__submit"
              disabled={!canSubmit}
              aria-label={t("home.hero.send")}
              onClick={() => start.mutate({})}
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
