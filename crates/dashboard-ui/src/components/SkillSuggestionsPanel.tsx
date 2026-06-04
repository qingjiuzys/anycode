import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

/** Compact banner; hidden when starter is complete and there is no usage signal. */
export function SkillSuggestionsPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [installMsg, setInstallMsg] = useState("");
  const q = useQuery({
    queryKey: ["skill-suggestions"],
    queryFn: api.skillSuggestions,
  });

  const installStarter = useMutation({
    mutationFn: () => api.installStarterSkills(),
    onSuccess: (data) => {
      void qc.invalidateQueries({ queryKey: ["skill-suggestions"] });
      void qc.invalidateQueries({ queryKey: ["skills"] });
      setInstallMsg(
        t("agents.installStarterOk").replace("{count}", String(data.count)),
      );
    },
    onError: (e: Error) => setInstallMsg(e.message),
  });

  const missing = q.data?.missing_starter ?? [];
  const usage = q.data?.usage ?? [];
  const hasActionable = missing.length > 0 || usage.length > 0;

  if (q.isLoading) return null;
  if (q.isError) {
    return (
      <div className="dw-agents-banner dw-agents-banner--error" role="status">
        <Icon name="error_outline" size={20} />
        <span>{t("agents.loadSuggestionsError")}</span>
      </div>
    );
  }

  if (!hasActionable && !installMsg) return null;

  return (
    <div className="dw-agents-banner" role="region" aria-label={t("agents.skillSuggestions")}>
      {missing.length > 0 && (
        <div className="dw-agents-banner__block">
          <div className="dw-agents-banner__lead">
            <Icon name="inventory_2" size={20} className="text-warn shrink-0" />
            <div>
              <p className="dw-agents-banner__title m-0">{t("agents.missingStarter")}</p>
              <p className="dw-agents-banner__sub m-0">
                {t("agents.missingStarterCount").replace("{n}", String(missing.length))}
              </p>
            </div>
          </div>
          <div className="dw-agents-banner__tags">
            {missing.map((id) => (
              <code key={id} className="font-code text-[11px] px-2 py-0.5 rounded-md bg-surface-container-high">
                {id}
              </code>
            ))}
          </div>
          <button
            type="button"
            className="dw-btn-primary text-sm shrink-0"
            disabled={installStarter.isPending}
            onClick={() => installStarter.mutate()}
          >
            <Icon name="download" size={16} />
            {installStarter.isPending ? t("agents.rescanning") : t("agents.installStarterBtn")}
          </button>
        </div>
      )}

      {usage.length > 0 && (
        <div className="dw-agents-banner__block dw-agents-banner__block--usage">
          <span className="text-xs font-medium text-secondary shrink-0">
            {t("agents.recentSkillUsage")}
          </span>
          <div className="dw-agents-banner__usage">
            {usage.slice(0, 6).map((row) => (
              <span key={row.skill_id} className="dw-agents-usage-pill font-code">
                {row.skill_id}
                <span className="text-secondary tabular-nums">{row.count}×</span>
              </span>
            ))}
          </div>
        </div>
      )}

      {installMsg && <p className="dw-agents-banner__msg m-0">{installMsg}</p>}

      <Link to="/settings" search={{ section: "skills" }} className="dw-agents-banner__link">
        {t("agents.skillsLink")}
        <Icon name="arrow_forward" size={14} />
      </Link>
    </div>
  );
}
