import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import brandLogo from "@/assets/anycode-logo-app-icon.png";
import { useT } from "@/i18n/context";

type Sse = "live" | "connecting" | "reconnecting" | "offline";

type PanelId = "recent" | "analytics" | "workbench";

const DISMISS_BROWSER_KEY = "anycode-home-browser-hint-dismiss";

export function HomeQuickCompose({
  sseStatus,
  projectOptions,
  activePanelId,
  onPanelChange,
  showRecentPanel,
}: {
  sseStatus: Sse;
  projectOptions: { id: string; name: string }[];
  activePanelId: string | null;
  onPanelChange: (id: PanelId | null) => void;
  showRecentPanel: boolean;
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
      void navigate({
        to: "/conversations",
        search: { session: data.session.id, project: projectId },
      });
    },
  });

  const connected = sseStatus === "live";
  const showBrowserRow =
    !browserHintDismissed &&
    browser.data?.bundled === true &&
    browser.data.enabled !== true;

  function panelIcon(id: PanelId): string {
    switch (id) {
      case "recent":
        return "history";
      case "analytics":
        return "analytics";
      case "workbench":
        return "dashboard_customize";
    }
  }

  function togglePanel(id: PanelId) {
    onPanelChange(activePanelId === id ? null : id);
  }

  function dismissBrowserHint() {
    sessionStorage.setItem(DISMISS_BROWSER_KEY, "1");
    setBrowserHintDismissed(true);
  }

  const canSubmit = prompt.trim().length > 0 && projectId.length > 0 && !start.isPending;

  return (
    <div className="dw-quick-compose">
      <div className="dw-quick-compose__input-row">
        <img src={brandLogo} alt="" className="dw-quick-compose__logo" />
        <input
          type="text"
          className="dw-quick-compose__input"
          placeholder={t("home.quickCompose.placeholder")}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && canSubmit) start.mutate();
          }}
        />
        <label className="dw-quick-compose__project-select">
          <span className="dw-quick-compose__new-chat">{t("home.quickCompose.newChat")}</span>
          <select
            value={projectId}
            onChange={(e) => setProjectId(e.target.value)}
            disabled={projectOptions.length === 0}
            className="dw-quick-compose__select"
            aria-label={t("home.quickCompose.newChat")}
          >
            {projectOptions.length === 0 ? (
              <option value="">{t("home.quickCompose.noProject")}</option>
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
        <button
          type="button"
          className="dw-quick-compose__submit"
          disabled={!canSubmit}
          aria-label={t("home.quickCompose.send")}
          onClick={() => start.mutate()}
        >
          <Icon name="arrow_upward" size={18} className="text-on-primary" />
        </button>
      </div>

      <div
        className={`dw-quick-compose__status ${connected ? "dw-quick-compose__status--ok" : "dw-quick-compose__status--warn"}`}
      >
        {connected
          ? t("home.quickCompose.statusLive")
          : sseStatus === "connecting" || sseStatus === "reconnecting"
            ? t("home.quickCompose.statusConnecting")
            : t("home.quickCompose.statusOffline")}
      </div>

      <div className="dw-quick-compose__footer">
        <div className="min-w-0">
          <p className="dw-quick-compose__footer-title">{t("home.quickCompose.shortcutsTitle")}</p>
          <p className="dw-quick-compose__footer-hint">
            {showBrowserRow
              ? t("home.quickCompose.shortcutsHintBrowser")
              : t("home.quickCompose.shortcutsHint")}
          </p>
        </div>
        <div className="dw-quick-compose__pills">
          {showBrowserRow && (
            <button
              type="button"
              className="dw-quick-compose__pill"
              disabled={enableBrowser.isPending}
              onClick={() => enableBrowser.mutate()}
            >
              {t("home.quickCompose.pillBrowser")}
            </button>
          )}
          {showRecentPanel && (
            <button
              type="button"
              className={`dw-quick-compose__pill ${activePanelId === "recent" ? "active" : ""}`}
              onClick={() => togglePanel("recent")}
            >
              <Icon name={panelIcon("recent")} size={14} />
              {t("home.recentSessions")}
            </button>
          )}
          <button
            type="button"
            className={`dw-quick-compose__pill ${activePanelId === "analytics" ? "active" : ""}`}
            onClick={() => togglePanel("analytics")}
          >
            <Icon name={panelIcon("analytics")} size={14} />
            {t("home.analyticsSection")}
          </button>
          <button
            type="button"
            className={`dw-quick-compose__pill ${activePanelId === "workbench" ? "active" : ""}`}
            onClick={() => togglePanel("workbench")}
          >
            <Icon name={panelIcon("workbench")} size={14} />
            {t("home.workbenchSection")}
          </button>
          {showBrowserRow && (
            <button type="button" className="dw-quick-compose__pill" onClick={dismissBrowserHint}>
              {t("home.quickCompose.pillNotNow")}
            </button>
          )}
        </div>
      </div>
      {start.isError && (
        <p className="text-xs text-error px-4 py-2 m-0 border-t border-outline-variant/60">
          {t("home.quickCompose.startError")}
        </p>
      )}
    </div>
  );
}
