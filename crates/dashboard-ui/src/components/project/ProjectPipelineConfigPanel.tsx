import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SessionFlow } from "@/components/SessionFlow";
import { useProjectViewPrefs } from "@/hooks/useProjectViewPrefs";
import { clampSessionFlowLimit } from "@/lib/projectViewPrefs";
import { useT } from "@/i18n/context";

export function ProjectPipelineConfigPanel({ projectId }: { projectId: string }) {
  const t = useT();
  const { prefs, update, savedFlash } = useProjectViewPrefs(projectId);

  const sessions = useQuery({
    queryKey: ["project-sessions-flow", projectId],
    queryFn: () => api.sessions(projectId),
    staleTime: 30_000,
  });

  const limit = clampSessionFlowLimit(prefs.sessionFlowLimit);

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-secondary m-0 inline-flex items-start gap-2">
        <Icon name="info" size={18} className="shrink-0 mt-0.5 text-secondary" />
        <span>{t("projectDetail.config.pipeline.intro")}</span>
      </p>

      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-3">
          <label className="text-sm font-medium text-on-surface" htmlFor="session-flow-limit">
            {t("projectDetail.config.pipeline.sessionLimit")}
          </label>
          <span className="text-xs text-secondary tabular-nums">
            {t("projectDetail.config.pipeline.sessionLimitValue").replace("{n}", String(limit))}
          </span>
        </div>
        <input
          id="session-flow-limit"
          type="range"
          min={3}
          max={20}
          step={1}
          className="w-full accent-primary"
          value={limit}
          onChange={(e) => update({ sessionFlowLimit: Number.parseInt(e.target.value, 10) })}
        />
        <span className="text-xs text-secondary">{t("projectDetail.config.pipeline.sessionLimitHint")}</span>
      </div>

      <div className="flex items-center justify-between gap-3 py-2 border-y border-outline-variant/60">
        <div>
          <p className="text-sm font-medium m-0">{t("projectDetail.config.pipeline.hideImported")}</p>
        </div>
        <label className="inline-flex items-center cursor-pointer">
          <input
            type="checkbox"
            className="accent-primary w-4 h-4"
            checked={prefs.hideImportedSessions}
            onChange={(e) => update({ hideImportedSessions: e.target.checked })}
          />
        </label>
      </div>

      {savedFlash && (
        <p className="text-xs text-secondary m-0">{t("projectDetail.config.autoSaved")}</p>
      )}

      <div>
        <p className="text-xs font-medium text-secondary mb-2 m-0">
          {t("projectDetail.config.pipeline.preview")}
        </p>
        <SessionFlow
          sessions={sessions.data?.sessions ?? []}
          limit={limit}
          hideImported={prefs.hideImportedSessions}
          preview
        />
      </div>
    </div>
  );
}
