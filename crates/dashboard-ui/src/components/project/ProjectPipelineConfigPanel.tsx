import { useProjectViewPrefs } from "@/hooks/useProjectViewPrefs";
import { useT } from "@/i18n/context";

export function ProjectPipelineConfigPanel({ projectId }: { projectId: string }) {
  const t = useT();
  const { prefs, update } = useProjectViewPrefs(projectId);

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-secondary m-0">{t("projectDetail.config.pipeline.intro")}</p>

      <label className="flex flex-col gap-1 text-sm">
        <span className="text-secondary font-medium">
          {t("projectDetail.config.pipeline.sessionLimit")}
        </span>
        <input
          type="number"
          min={3}
          max={20}
          className="dw-input w-24"
          value={prefs.sessionFlowLimit}
          onChange={(e) =>
            update({ sessionFlowLimit: Number.parseInt(e.target.value, 10) || 8 })
          }
        />
        <span className="text-xs text-secondary">{t("projectDetail.config.pipeline.sessionLimitHint")}</span>
      </label>

      <label className="text-sm text-secondary inline-flex items-center gap-2">
        <input
          type="checkbox"
          className="accent-primary"
          checked={prefs.hideImportedSessions}
          onChange={(e) => update({ hideImportedSessions: e.target.checked })}
        />
        {t("projectDetail.config.pipeline.hideImported")}
      </label>
    </div>
  );
}
