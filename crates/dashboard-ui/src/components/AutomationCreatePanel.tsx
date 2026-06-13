import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useT } from "@/i18n/context";

const TIMEZONES = [
  { id: "local", label: "Local" },
  { id: "Asia/Shanghai", label: "Asia/Shanghai" },
  { id: "UTC", label: "UTC" },
  { id: "America/New_York", label: "America/New_York" },
  { id: "Europe/London", label: "Europe/London" },
] as const;

const TOOL_PROFILES = ["default", "read_only", "observability", "allowlist"] as const;

function AutomationCreateForm({
  defaultProjectId = "",
  onCreated,
}: {
  /** Pre-selected project when opened from a project context; "" = whole workspace. */
  defaultProjectId?: string;
  onCreated?: () => void;
}) {
  const t = useT();
  const qc = useQueryClient();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [schedule, setSchedule] = useState("0 0 8 * * *");
  const [naturalSchedule, setNaturalSchedule] = useState("");
  const [command, setCommand] = useState("");
  const [scheduleTimezone, setScheduleTimezone] = useState("local");
  const [toolProfile, setToolProfile] = useState<(typeof TOOL_PROFILES)[number]>("observability");
  const [projectId, setProjectId] = useState(defaultProjectId);
  const [sessionId, setSessionId] = useState("");
  const [failureDestination, setFailureDestination] = useState("");
  const [msg, setMsg] = useState("");

  const templates = useQuery({
    queryKey: ["cron-templates"],
    queryFn: api.cronTemplates,
  });

  const projects = useQuery({
    queryKey: ["projects"],
    queryFn: () => api.projects({ limit: 500 }),
  });

  const parseSchedule = useMutation({
    mutationFn: () => api.parseCronSchedule(naturalSchedule.trim()),
    onSuccess: (data) => {
      setSchedule(data.schedule);
      setMsg(data.summary);
    },
    onError: (e: Error) => setMsg(e.message),
  });

  const create = useMutation({
    mutationFn: () =>
      api.createCronJob({
        schedule,
        command,
        schedule_timezone: scheduleTimezone,
        tool_profile: toolProfile,
        session_id: sessionId.trim() || undefined,
        failure_destination: failureDestination.trim() || undefined,
        project_id: projectId || undefined,
      }),
    onSuccess: () => {
      setMsg(t("automations.createOk"));
      void qc.invalidateQueries({ queryKey: ["cron-jobs"] });
      onCreated?.();
    },
    onError: (e: Error) => setMsg(e.message),
  });

  return (
    <>
      {(templates.data?.templates ?? []).length > 0 && (
        <div className="flex flex-wrap gap-2 mb-3">
          {(templates.data?.templates ?? []).map((tpl) => {
            const id = String((tpl as { id?: string }).id ?? "");
            return (
              <button
                key={id}
                type="button"
                className="dw-btn-secondary text-xs"
                onClick={() => {
                  const tplRow = tpl as {
                    schedule?: string;
                    command?: string;
                    schedule_timezone?: string;
                    tool_profile?: string;
                  };
                  if (tplRow.schedule) setSchedule(tplRow.schedule);
                  if (tplRow.command) setCommand(tplRow.command);
                  if (tplRow.schedule_timezone) setScheduleTimezone(tplRow.schedule_timezone);
                  if (
                    tplRow.tool_profile &&
                    TOOL_PROFILES.includes(tplRow.tool_profile as (typeof TOOL_PROFILES)[number])
                  ) {
                    setToolProfile(tplRow.tool_profile as (typeof TOOL_PROFILES)[number]);
                  }
                }}
              >
                {String((tpl as { name?: string }).name ?? id)}
              </button>
            );
          })}
        </div>
      )}
      <label className="block text-xs text-secondary mb-1">{t("automations.naturalSchedule")}</label>
      <div className="flex flex-wrap gap-2 mb-3">
        <input
          className="dw-input flex-1 min-w-[12rem] text-sm"
          placeholder={t("automations.naturalSchedulePlaceholder")}
          value={naturalSchedule}
          onChange={(e) => setNaturalSchedule(e.target.value)}
        />
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          disabled={!naturalSchedule.trim() || parseSchedule.isPending}
          onClick={() => parseSchedule.mutate()}
        >
          {parseSchedule.isPending ? "…" : t("automations.parseSchedule")}
        </button>
      </div>
      <label className="block text-xs text-secondary mb-1">{t("automations.commandSummary")}</label>
      <textarea
        className="dw-input w-full min-h-[96px] mb-3"
        placeholder={t("automations.commandPlaceholder")}
        value={command}
        onChange={(e) => setCommand(e.target.value)}
      />
      <label className="block text-xs text-secondary mb-1">{t("automations.jobProject")}</label>
      <select
        className="dw-input w-full mb-4"
        value={projectId}
        onChange={(e) => setProjectId(e.target.value)}
      >
        <option value="">{t("automations.wholeWorkspace")}</option>
        {(projects.data?.projects ?? []).map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      {advancedOpen && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-4">
          <div>
            <label className="block text-xs text-secondary mb-1">{t("automations.schedule")}</label>
            <input
              className="dw-input w-full font-code text-sm"
              value={schedule}
              onChange={(e) => setSchedule(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-xs text-secondary mb-1">{t("automations.timezone")}</label>
            <select
              className="dw-input w-full"
              value={scheduleTimezone}
              onChange={(e) => setScheduleTimezone(e.target.value)}
            >
              {TIMEZONES.map((tz) => (
                <option key={tz.id} value={tz.id}>
                  {tz.label}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-secondary mb-1">{t("automations.toolProfile")}</label>
            <select
              className="dw-input w-full"
              value={toolProfile}
              onChange={(e) => setToolProfile(e.target.value as (typeof TOOL_PROFILES)[number])}
            >
              {TOOL_PROFILES.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-secondary mb-1">{t("automations.stableSession")}</label>
            <input
              className="dw-input w-full font-code text-sm"
              placeholder={t("automations.stableSessionPlaceholder")}
              value={sessionId}
              onChange={(e) => setSessionId(e.target.value)}
            />
          </div>
          <div className="md:col-span-2">
            <label className="block text-xs text-secondary mb-1">{t("automations.failureDestination")}</label>
            <input
              className="dw-input w-full text-sm"
              placeholder={t("automations.failureDestinationPlaceholder")}
              value={failureDestination}
              onChange={(e) => setFailureDestination(e.target.value)}
            />
          </div>
        </div>
      )}
      <div className="flex flex-wrap items-center justify-between gap-3 pt-1">
        <button
          type="button"
          className="inline-flex items-center gap-1 border-0 bg-transparent p-0 text-xs text-secondary hover:text-primary cursor-pointer"
          onClick={() => setAdvancedOpen((v) => !v)}
          aria-expanded={advancedOpen}
        >
          <Icon name={advancedOpen ? "expand_less" : "expand_more"} size={16} />
          {advancedOpen ? t("automations.hideAdvanced") : t("automations.showAdvanced")}
        </button>
        <button
          type="button"
          className="dw-btn-primary shrink-0"
          disabled={!command.trim() || create.isPending}
          onClick={() => create.mutate()}
        >
          {create.isPending ? "…" : t("automations.createBtn")}
        </button>
      </div>
      {msg && <p className="text-sm text-secondary mt-3 m-0">{msg}</p>}
    </>
  );
}

export function AutomationCreatePanel({ defaultProjectId }: { defaultProjectId?: string }) {
  const t = useT();
  const [open, setOpen] = useState(true);

  return (
    <SectionCard
      title={t("automations.createTitle")}
      action={
        <button
          type="button"
          className="inline-flex items-center gap-1 border-0 bg-transparent p-0 text-xs text-secondary hover:text-primary cursor-pointer"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
        >
          <Icon name={open ? "expand_less" : "expand_more"} size={16} />
          {open ? t("automations.collapseForm") : t("automations.expandForm")}
        </button>
      }
    >
      {open ? (
        <AutomationCreateForm defaultProjectId={defaultProjectId} />
      ) : (
        <p className="text-sm text-secondary m-0">{t("automations.createCollapsedHint")}</p>
      )}
    </SectionCard>
  );
}

export function AutomationCreateDialog({
  open,
  onClose,
  defaultProjectId,
}: {
  open: boolean;
  onClose: () => void;
  defaultProjectId?: string;
}) {
  const t = useT();
  if (!open) return null;
  return (
    <ModalOverlay open={open} onClose={onClose} labelledBy="new-automation-title" className="w-full max-w-lg">
      <div className="glass-modal rounded-xl p-6 max-h-[min(90dvh,720px)] overflow-y-auto">
        <div className="flex items-start justify-between gap-4 mb-4">
          <h2 id="new-automation-title" className="text-lg font-semibold m-0 text-on-surface">
            {t("automations.createTitle")}
          </h2>
          <button
            type="button"
            className="dw-btn-ghost p-1"
            onClick={onClose}
            aria-label={t("newProject.cancel")}
          >
            <Icon name="close" size={20} />
          </button>
        </div>
        <AutomationCreateForm defaultProjectId={defaultProjectId} onCreated={onClose} />
      </div>
    </ModalOverlay>
  );
}
