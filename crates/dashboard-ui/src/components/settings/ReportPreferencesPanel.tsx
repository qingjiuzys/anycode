import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { useEffect, useState } from "react";

export type ReportOutputFormat = "markdown" | "html" | "both";
export type ReportGenerationMode = "llm" | "template";

export function ReportPreferencesPanel() {
  const t = useT();
  const qc = useQueryClient();
  const prefs = useQuery({
    queryKey: ["dashboard-preferences"],
    queryFn: api.dashboardPreferences,
  });
  const runtime = useQuery({
    queryKey: ["runtime-settings"],
    queryFn: api.runtimeSettings,
  });
  const view = prefs.data?.preferences;
  const src = view?.saved ?? view?.active;

  const [outputFormat, setOutputFormat] = useState<ReportOutputFormat>("markdown");
  const [generationMode, setGenerationMode] = useState<ReportGenerationMode>("llm");

  useEffect(() => {
    if (!src) return;
    const fmt = src.report_output_format;
    if (fmt === "html" || fmt === "both") {
      setOutputFormat(fmt);
    } else {
      setOutputFormat("markdown");
    }
    setGenerationMode(src.report_generation_mode === "template" ? "template" : "llm");
  }, [src]);

  const save = useMutation({
    mutationFn: () => {
      if (!src) throw new Error("preferences not loaded");
      return api.saveDashboardPreferences({
        host: src.host,
        port: src.port,
        db_path: src.db_path,
        asset_read_strict: Boolean(src.asset_read_strict),
        report_output_format: outputFormat,
        report_generation_mode: generationMode,
      });
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["dashboard-preferences"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const chatConfigured =
    Boolean(runtime.data?.runtime.global_model) ||
    Boolean(runtime.data?.runtime.routing_agents?.length);

  return (
    <SectionCard title={t("settings.reportPrefs.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.reportPrefs.hint")}</p>

      {!chatConfigured && generationMode === "llm" && (
        <p className="text-sm text-warning m-0 mb-4 rounded-lg bg-warning-container/20 px-3 py-2">
          {t("settings.reportPrefs.llmNotConfigured")}
        </p>
      )}

      <fieldset className="border-0 p-0 m-0 mb-4">
        <legend className="text-sm font-medium text-on-surface mb-2">
          {t("settings.reportPrefs.outputFormat")}
        </legend>
        <div className="flex flex-wrap gap-2">
          {(["markdown", "html", "both"] as ReportOutputFormat[]).map((id) => (
            <label key={id} className="inline-flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="radio"
                name="report_output_format"
                checked={outputFormat === id}
                onChange={() => setOutputFormat(id)}
              />
              {t(`settings.reportPrefs.format.${id}`)}
            </label>
          ))}
        </div>
      </fieldset>

      <fieldset className="border-0 p-0 m-0 mb-4">
        <legend className="text-sm font-medium text-on-surface mb-2">
          {t("settings.reportPrefs.generationMode")}
        </legend>
        <div className="flex flex-wrap gap-4">
          <label className="inline-flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="report_generation_mode"
              checked={generationMode === "llm"}
              onChange={() => setGenerationMode("llm")}
            />
            {t("settings.reportPrefs.mode.llm")}
          </label>
          <label className="inline-flex items-center gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="report_generation_mode"
              checked={generationMode === "template"}
              onChange={() => setGenerationMode("template")}
            />
            {t("settings.reportPrefs.mode.template")}
          </label>
        </div>
        <p className="text-xs text-secondary m-0 mt-2">{t("settings.reportPrefs.modeHint")}</p>
      </fieldset>

      <button
        type="button"
        className="dw-btn-primary"
        disabled={save.isPending || !src}
        onClick={() => save.mutate()}
      >
        {save.isPending ? t("settings.prefs.save") : t("settings.reportPrefs.save")}
      </button>
      {save.isError && (
        <p className="text-sm text-error m-0 mt-2">{(save.error as Error).message}</p>
      )}
    </SectionCard>
  );
}
