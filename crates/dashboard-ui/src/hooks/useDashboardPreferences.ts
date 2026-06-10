import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { DashboardPreferencesView } from "@/api/types";
import {
  buildSaveDashboardPreferencesBody,
  preferencesSource,
} from "@/lib/dashboardPreferences";
import type { DashboardPreferences } from "@/api/types";

type PreferencesQuery = { preferences: DashboardPreferencesView };

export function useDashboardPreferences() {
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: ["dashboard-preferences"],
    queryFn: api.dashboardPreferences,
  });
  const view = query.data?.preferences;
  const src = preferencesSource(view);

  const save = useMutation({
    mutationFn: (
      patch: Partial<{
        host: string;
        port: number;
        db_path: string;
        asset_read_strict: boolean;
        report_output_format: DashboardPreferences["report_output_format"];
        report_generation_mode: DashboardPreferences["report_generation_mode"];
      }>,
    ) => {
      const cached = qc.getQueryData<PreferencesQuery>(["dashboard-preferences"]);
      const base = preferencesSource(cached?.preferences) ?? src;
      if (!base) {
        throw new Error("preferences not loaded");
      }
      return api.saveDashboardPreferences(buildSaveDashboardPreferencesBody(base, patch));
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["dashboard-preferences"] });
      qc.invalidateQueries({ queryKey: ["runtime-settings"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  return { query, view, src, save };
}
