import { SectionCard } from "@/components/ui/SectionCard";
import { useDashboardPreferences } from "@/hooks/useDashboardPreferences";
import { useT } from "@/i18n/context";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { useEffect, useState } from "react";

export type ReportOutputFormat = "markdown" | "html" | "both";
export type ReportGenerationMode = "llm" | "template";

export function ReportPreferencesPanel() {
  const t = useT();
  const { src, save, query } = useDashboardPreferences();
  const runtime = useQuery({
    queryKey: ["runtime-settings"],
    queryFn: api.runtimeSettings,
  });

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

  const chatConfigured =
    Boolean(runtime.data?.runtime.global_model) ||
    Boolean(runtime.data?.runtime.routing_agents?.length);

  const canSave = Boolean(src) && !save.isPending;

  return (
    <SectionCard title={t("settings.reportPrefs.title")}>
      <p className="text-sm text-secondary m-0 mb-4">{t("settings.reportPrefs.hint")}</p>

      {query.isLoading && (
        <p className="text-sm text-secondary m-0 mb-4">{t("common.loading")}</p>
      )}

      {!query.isLoading && !src && (
        <p className="text-sm text-error m-0 mb-4">{t("settings.userPrefs.loadFailed")}</p>
      )}

      {!chatConfigured && generationMode === "llm" && (
        <p className="text-sm text-warning m-0 mb-4 rounded-lg bg-warning-container/20 px-3 py-2">
          {t("settings.reportPrefs.llmNotConfigured")}
        </p>
      )}

      <fieldset className="border-0 p-0 m-0 mb-4" disabled={!src}>
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

      <fieldset className="border-0 p-0 m-0 mb-4" disabled={!src}>
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

      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          className="dw-btn-primary"
          disabled={!canSave}
          onClick={() =>
            save.mutate({
              report_output_format: outputFormat,
              report_generation_mode: generationMode,
            })
          }
        >
          {save.isPending ? t("common.loading") : t("settings.reportPrefs.save")}
        </button>
        {save.isSuccess && (
          <span className="text-sm text-secondary">{t("settings.userPrefs.saved")}</span>
        )}
      </div>
      {save.isError && (
        <p className="text-sm text-error m-0 mt-2">{(save.error as Error).message}</p>
      )}
    </SectionCard>
  );
}
