import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

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

  if (!q.isLoading && missing.length === 0 && usage.length === 0 && !installMsg) {
    return null;
  }

  return (
    <SectionCard title={t("agents.skillSuggestions")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("agents.skillSuggestionsHint")}</p>
      {missing.length > 0 && (
        <div className="rounded-xl bg-surface-container-low p-4 mb-3">
          <div className="text-xs font-semibold text-on-surface-variant mb-2">
            {t("agents.missingStarter")}
          </div>
          <div className="flex flex-wrap gap-2 mb-3">
            {missing.map((id) => (
              <code
                key={id}
                className="font-code text-xs bg-surface-container-high px-2 py-1 rounded"
              >
                {id}
              </code>
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              className="dw-btn-primary text-sm"
              disabled={installStarter.isPending}
              onClick={() => installStarter.mutate()}
            >
              {installStarter.isPending ? "…" : t("agents.installStarterBtn")}
            </button>
            <span className="text-xs text-secondary">
              {t("agents.missingStarterHint")}{" "}
              <code className="font-code">anycode skills install-starter</code>
            </span>
          </div>
        </div>
      )}
      {usage.length > 0 && (
        <div className="rounded-xl bg-surface-container-low p-4">
          <div className="text-xs font-semibold text-on-surface-variant mb-2">
            {t("agents.recentSkillUsage")}
          </div>
          <ul className="list-none m-0 p-0 space-y-2">
            {usage.map((row) => (
              <li
                key={row.skill_id}
                className="flex items-center justify-between gap-3 text-sm py-1.5 px-2 rounded-lg bg-surface-container-lowest/70"
              >
                <span className="font-code truncate">{row.skill_id}</span>
                <span className="text-secondary tabular-nums shrink-0">{row.count}×</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {installMsg && <p className="text-sm text-secondary mt-3 m-0">{installMsg}</p>}
      <div className="dw-inline-links mt-4 pt-3 border-t border-outline-variant/50">
        <Link to="/settings" search={{ section: "skills" }} className="dw-inline-link">
          <Icon name="extension" size={16} />
          {t("agents.skillsLink")}
        </Link>
      </div>
    </SectionCard>
  );
}
