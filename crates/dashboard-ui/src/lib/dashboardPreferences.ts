import type {
  DashboardPreferences,
  DashboardPreferencesView,
  ReportGenerationMode,
  ReportOutputFormat,
} from "@/api/types";

export type SaveDashboardPreferencesBody = {
  host: string;
  port: number;
  db_path: string;
  asset_read_strict?: boolean;
  report_output_format?: string;
  report_generation_mode?: string;
};

export function preferencesSource(
  view: DashboardPreferencesView | undefined,
): DashboardPreferences | undefined {
  if (!view) return undefined;
  return view.saved ?? view.active;
}

export function buildSaveDashboardPreferencesBody(
  src: DashboardPreferences,
  patch: Partial<{
    host: string;
    port: number;
    db_path: string;
    asset_read_strict: boolean;
    report_output_format: ReportOutputFormat;
    report_generation_mode: ReportGenerationMode;
  }>,
): SaveDashboardPreferencesBody {
  return {
    host: (patch.host ?? src.host).trim(),
    port: patch.port ?? src.port,
    db_path: (patch.db_path ?? src.db_path).trim(),
    asset_read_strict: patch.asset_read_strict ?? Boolean(src.asset_read_strict),
    report_output_format: patch.report_output_format ?? src.report_output_format ?? "markdown",
    report_generation_mode: patch.report_generation_mode ?? src.report_generation_mode ?? "llm",
  };
}
