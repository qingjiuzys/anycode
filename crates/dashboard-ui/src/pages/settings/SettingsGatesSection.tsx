import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { SectionCard } from "@/components/ui/SectionCard";
import { useI18n } from "@/i18n/context";
import { projectGatesDocsUrl } from "@/lib/docLinks";
import { get, put } from "@/api/http";

const STATIC_PRESET_IDS = [
  "cargo_fmt",
  "cargo_clippy",
  "cargo_test",
  "npm_test",
  "playwright",
  "flutter_analyze",
  "flutter_test",
  "project_verify",
  "go_test",
  "pytest",
] as const;

type GatePrefs = {
  acceptance_gates_default: boolean;
  default_acceptance_preset_ids: string[];
};

export function SettingsGatesSection() {
  const { t, locale } = useI18n();
  const queryClient = useQueryClient();

  const gatePrefs = useQuery({
    queryKey: ["gate-prefs"],
    queryFn: () => get<GatePrefs>("/api/settings/gate-prefs"),
  });

  const savePrefs = useMutation({
    mutationFn: (body: GatePrefs) => put<{ ok: boolean }>("/api/settings/gate-prefs", body),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["gate-prefs"] });
    },
  });

  const projects = useQuery({
    queryKey: ["projects", "gates-settings"],
    queryFn: () => api.projects({ limit: 1 }),
    staleTime: 60_000,
  });

  const sampleProjectId = projects.data?.projects?.[0]?.id;
  const presets = useQuery({
    queryKey: ["gate-presets", "settings-sample", sampleProjectId],
    queryFn: () => api.gatePresets(sampleProjectId!),
    enabled: Boolean(sampleProjectId),
    staleTime: 60_000,
  });

  const detectedRows = presets.data?.presets ?? [];
  const useStaticList = detectedRows.length === 0;
  const acceptanceDefault = gatePrefs.data?.acceptance_gates_default ?? false;

  function onAcceptanceDefaultChange(checked: boolean) {
    const presetIds =
      gatePrefs.data?.default_acceptance_preset_ids ??
      (useStaticList
        ? [...STATIC_PRESET_IDS]
        : detectedRows.map((r) => r.id));
    savePrefs.mutate({
      acceptance_gates_default: checked,
      default_acceptance_preset_ids: presetIds,
    });
  }

  return (
    <>
      <SectionCard title={t("settings.gates.title")}>
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.gates.intro")}</p>
        <p className="text-sm text-secondary m-0 mb-4">{t("settings.gates.perProjectHint")}</p>
        <Link to="/projects" className="text-sm text-primary no-underline hover:underline">
          {t("settings.gates.openProjects")}
        </Link>
      </SectionCard>

      <SectionCard title={t("settings.gates.globalDefaultTitle")}>
        <label className="inline-flex items-start gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            className="mt-0.5"
            checked={acceptanceDefault}
            disabled={gatePrefs.isLoading || savePrefs.isPending}
            onChange={(e) => onAcceptanceDefaultChange(e.target.checked)}
          />
          <span>
            <span className="font-medium text-on-surface block">
              {t("settings.gates.acceptanceDefault")}
            </span>
            <span className="text-secondary text-xs">{t("settings.gates.acceptanceDefaultHint")}</span>
          </span>
        </label>
      </SectionCard>

      <SectionCard title={t("settings.gates.presetTypesTitle")}>
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.gates.presetTypesHint")}</p>
        {useStaticList ? (
          <p className="text-xs text-secondary m-0 mb-3">{t("settings.gates.staticListHint")}</p>
        ) : (
          <p className="text-xs text-secondary m-0 mb-3">
            {t("settings.gates.detectedFromProject").replace(
              "{name}",
              projects.data?.projects?.[0]?.name ?? sampleProjectId ?? "",
            )}
          </p>
        )}
        <ul className="text-sm m-0 pl-5 space-y-1">
          {(useStaticList
            ? STATIC_PRESET_IDS.map((id) => ({
                id,
                name: t(`settings.gates.staticPresets.${id}` as "settings.gates.title"),
                command: "",
              }))
            : detectedRows
          ).map((row) => (
            <li key={row.id}>
              <span className="font-medium">{row.name}</span>
              {row.command ? (
                <span className="text-secondary text-xs ml-2 font-code">{row.command}</span>
              ) : null}
            </li>
          ))}
        </ul>
      </SectionCard>

      <SectionCard title={t("settings.gates.docsTitle")}>
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.gates.docsHint")}</p>
        <ExternalNavLink href={projectGatesDocsUrl(locale)} className="dw-btn-secondary no-underline">
          {t("settings.gates.openDocs")}
        </ExternalNavLink>
      </SectionCard>
    </>
  );
}
