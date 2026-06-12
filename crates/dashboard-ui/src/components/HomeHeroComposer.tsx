import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { buildConversationsHref, conversationSearchParams } from "@/lib/conversationsSearch";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { mergeVoiceTranscript, VoiceInputButton } from "@/components/VoiceInputButton";
import { useT } from "@/i18n/context";

type Sse = "live" | "connecting" | "reconnecting" | "offline";

const DISMISS_BROWSER_KEY = "anycode-home-browser-hint-dismiss";

export function HomeHeroComposer({
  sseStatus,
  projectOptions,
  blockedCount = 0,
  pendingCount = 0,
  budgetExceededCount = 0,
}: {
  sseStatus: Sse;
  projectOptions: { id: string; name: string }[];
  blockedCount?: number;
  pendingCount?: number;
  budgetExceededCount?: number;
}) {
  const t = useT();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [prompt, setPrompt] = useState("");
  const [projectId, setProjectId] = useState("");
  const [browserHintDismissed, setBrowserHintDismissed] = useState(false);

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

  useEffect(() => {
    if (!projectId && projectOptions.length > 0) {
      setProjectId(projectOptions[0].id);
    }
  }, [projectId, projectOptions]);

  const start = useMutation({
    mutationFn: () =>
      api.startConversation(projectId, {
        prompt: prompt.trim(),
      }),
    onSuccess: (data) => {
      setPrompt("");
      const canon = conversationSearchParams({
        session: data.session.id,
        project: projectId,
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

  const canSubmit = prompt.trim().length > 0 && projectId.length > 0 && !start.isPending;
  const hasAlerts = blockedCount > 0 || pendingCount > 0 || budgetExceededCount > 0;

  const statusLabel = connected
    ? t("home.hero.statusLive")
    : sseStatus === "connecting" || sseStatus === "reconnecting"
      ? t("home.hero.statusConnecting")
      : t("home.hero.statusOffline");

  return (
    <div className="dw-hero-composer">
      <div className="dw-hero-composer__card">
        <textarea
          className="dw-hero-composer__textarea"
          placeholder={t("home.hero.placeholder")}
          value={prompt}
          rows={3}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && canSubmit) {
              e.preventDefault();
              start.mutate();
            }
          }}
        />
        <div className="dw-hero-composer__toolbar">
          <label className="dw-hero-composer__project-select">
            <Icon name="folder" size={16} className="text-secondary shrink-0" />
            <select
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
              disabled={projectOptions.length === 0}
              className="dw-hero-composer__select"
              aria-label={t("home.hero.projectLabel")}
            >
              {projectOptions.length === 0 ? (
                <option value="">{t("home.hero.noProject")}</option>
              ) : (
                projectOptions.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))
              )}
            </select>
            <Icon name="expand_more" size={14} className="text-secondary shrink-0 pointer-events-none" />
          </label>
          <div className="flex items-center gap-2 shrink-0 ml-auto">
            <VoiceInputButton
              disabled={start.isPending}
              onTranscribed={(text) => setPrompt((prev) => mergeVoiceTranscript(prev, text))}
            />
            <button
              type="button"
              className="dw-hero-composer__submit"
              disabled={!canSubmit}
              aria-label={t("home.hero.send")}
              onClick={() => start.mutate()}
            >
              <Icon name="arrow_upward" size={20} className="text-on-primary" />
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
        <p className="text-xs text-error m-0 text-center">{t("home.hero.startError")}</p>
      )}
    </div>
  );
}
