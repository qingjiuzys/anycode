import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { MemoryRetentionPanel } from "@/components/settings/MemoryRetentionPanel";
import { SectionCard } from "@/components/ui/SectionCard";
import { useRuntimeSettings } from "@/hooks/useRuntimeSettings";
import { useT } from "@/i18n/context";

export function SettingsDataSection() {
  const t = useT();
  const qc = useQueryClient();
  const health = useQuery({ queryKey: ["health"], queryFn: api.health });
  const db = useQuery({ queryKey: ["database"], queryFn: api.database });
  const dbOps = useQuery({ queryKey: ["db-operations"], queryFn: api.dbOperations });
  const runtime = useRuntimeSettings();
  const rt = runtime.data?.runtime;
  const backup = useMutation({
    mutationFn: () => api.databaseBackup(),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["db-operations"] });
    },
  });

  return (
    <>
      <SectionCard title={t("settings.database")}>
        <dl className="grid grid-cols-[minmax(3rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
          <dt className="text-secondary font-medium m-0">{t("settings.path")}</dt>
          <dd className="m-0 font-code text-xs text-secondary break-all">
            {db.data?.path ?? health.data?.db_path ?? rt?.db_path}
          </dd>
          <dt className="text-secondary font-medium m-0">{t("settings.driver")}</dt>
          <dd className="m-0">{db.data?.driver ?? "sqlite"}</dd>
        </dl>
      </SectionCard>
      <SectionCard title={t("settings.dataOps")}>
        {dbOps.data?.operations && (
          <>
            <p className="text-sm m-0 mb-2">
              {t("settings.size")}:{" "}
              {(dbOps.data.operations.db_size_bytes / 1_048_576).toFixed(2)} MB ·{" "}
              {t("settings.migrations")}: {dbOps.data.operations.migrations.length} ·{" "}
              {t("settings.health")}: {dbOps.data.operations.health_status}
            </p>
            <p className="text-sm text-secondary m-0 mb-2">
              {t("settings.backup")}: {dbOps.data.operations.backup_suggestion}
            </p>
            <button
              type="button"
              className="dw-btn dw-btn-secondary text-sm mb-2"
              disabled={backup.isPending}
              onClick={() => backup.mutate()}
            >
              {backup.isPending ? t("settings.backupRunning") : t("settings.backupNow")}
            </button>
            {backup.data?.ok && (
              <p className="text-sm text-secondary m-0 mb-2">{backup.data.path}</p>
            )}
            {backup.data?.error && (
              <p className="text-sm text-error m-0 mb-2">{backup.data.error}</p>
            )}
            {dbOps.data.operations.growth_warnings.map((w) => (
              <p key={w} className="text-sm text-secondary m-0 mb-1">
                ⚠ {w}
              </p>
            ))}
            <p className="text-sm text-secondary m-0">
              Workbench: Settings → Data · DB health and backup via API
            </p>
          </>
        )}
      </SectionCard>
      <MemoryRetentionPanel />
    </>
  );
}
