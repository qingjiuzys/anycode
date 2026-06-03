import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const TIMEZONES = [
  { id: "local", label: "Local" },
  { id: "Asia/Shanghai", label: "Asia/Shanghai" },
  { id: "UTC", label: "UTC" },
  { id: "America/New_York", label: "America/New_York" },
  { id: "Europe/London", label: "Europe/London" },
] as const;

const TOOL_PROFILES = ["default", "read_only", "observability", "allowlist"] as const;

export function AutomationCreatePanel() {
  const t = useT();
  const qc = useQueryClient();
  const [open, setOpen] = useState(false);
  const [schedule, setSchedule] = useState("0 0 8 * * *");
  const [naturalSchedule, setNaturalSchedule] = useState("");
  const [command, setCommand] = useState("");
  const [scheduleTimezone, setScheduleTimezone] = useState("local");
  const [toolProfile, setToolProfile] = useState<(typeof TOOL_PROFILES)[number]>("observability");
  const [sessionId, setSessionId] = useState("");
  const [failureDestination, setFailureDestination] = useState("");
  const [msg, setMsg] = useState("");

  const templates = useQuery({
    queryKey: ["cron-templates"],
    queryFn: api.cronTemplates,
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
      }),
    onSuccess: () => {
      setMsg(t("automations.createOk"));
      void qc.invalidateQueries({ queryKey: ["cron-jobs"] });
    },
    onError: (e: Error) => setMsg(e.message),
  });

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
        <>
      <p className="text-sm text-secondary m-0 mb-3">{t("automations.createHint")}</p>
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
      <div className="flex flex-wrap gap-2 mb-2">
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
      <label className="block text-xs text-secondary mb-1">{t("automations.schedule")}</label>
      <input
        className="dw-input w-full mb-2 font-code text-sm"
        value={schedule}
        onChange={(e) => setSchedule(e.target.value)}
      />
      <label className="block text-xs text-secondary mb-1">{t("automations.timezone")}</label>
      <select
        className="dw-input w-full mb-2"
        value={scheduleTimezone}
        onChange={(e) => setScheduleTimezone(e.target.value)}
      >
        {TIMEZONES.map((tz) => (
          <option key={tz.id} value={tz.id}>
            {tz.label}
          </option>
        ))}
      </select>
      <label className="block text-xs text-secondary mb-1">{t("automations.toolProfile")}</label>
      <select
        className="dw-input w-full mb-2"
        value={toolProfile}
        onChange={(e) => setToolProfile(e.target.value as (typeof TOOL_PROFILES)[number])}
      >
        {TOOL_PROFILES.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>
      <label className="block text-xs text-secondary mb-1">{t("automations.stableSession")}</label>
      <input
        className="dw-input w-full mb-2 font-code text-sm"
        placeholder={t("automations.stableSessionPlaceholder")}
        value={sessionId}
        onChange={(e) => setSessionId(e.target.value)}
      />
      <label className="block text-xs text-secondary mb-1">{t("automations.failureDestination")}</label>
      <input
        className="dw-input w-full mb-2 text-sm"
        placeholder={t("automations.failureDestinationPlaceholder")}
        value={failureDestination}
        onChange={(e) => setFailureDestination(e.target.value)}
      />
      <label className="block text-xs text-secondary mb-1">{t("automations.commandSummary")}</label>
      <textarea
        className="dw-input w-full min-h-[80px] mb-2"
        value={command}
        onChange={(e) => setCommand(e.target.value)}
      />
      <button
        type="button"
        className="dw-btn-primary"
        disabled={!command.trim() || create.isPending}
        onClick={() => create.mutate()}
      >
        {create.isPending ? "…" : t("automations.createBtn")}
      </button>
      {msg && <p className="text-sm text-secondary mt-2 m-0">{msg}</p>}
        </>
      ) : (
        <p className="text-sm text-secondary m-0">{t("automations.createCollapsedHint")}</p>
      )}
    </SectionCard>
  );
}
