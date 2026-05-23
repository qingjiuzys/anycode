import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

const PRESET_EVENTS = [
  "session_report_generated",
  "project_report_generated",
  "gate_failed",
  "session_blocked",
  "blocked_threshold_exceeded",
] as const;

export function NotificationPoliciesPanel() {
  const t = useT();
  const qc = useQueryClient();
  const [notifyEvent, setNotifyEvent] = useState("session_report_generated");

  const policies = useQuery({
    queryKey: ["notifications"],
    queryFn: () => api.notificationPolicies(),
  });

  const addLocalLogPolicy = useMutation({
    mutationFn: () =>
      api.upsertNotificationPolicy({
        event_type: notifyEvent,
        channel: "local_log",
        config: {},
        enabled: true,
      }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["notifications"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const testNotify = useMutation({
    mutationFn: () => api.testNotification(notifyEvent),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["notifications-audit"] }),
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteNotificationPolicy(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["notifications"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
    },
  });

  const toggle = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      api.setNotificationPolicyEnabled(id, enabled),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["notifications"] }),
  });

  const rows = policies.data?.policies ?? [];

  return (
    <SectionCard title={t("settings.notifications")} noPadding>
      <div className="px-4 pt-4 pb-3 border-b border-outline-variant">
        <p className="text-sm text-secondary m-0 mb-3">{t("settings.notificationsHint")}</p>
        <div className="flex flex-wrap gap-2 mb-2">
          {PRESET_EVENTS.map((ev) => (
            <button
              key={ev}
              type="button"
              className={`dw-chip${notifyEvent === ev ? " active" : ""}`}
              onClick={() => setNotifyEvent(ev)}
            >
              {ev}
            </button>
          ))}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <input
            className="dw-input flex-1 min-w-[12rem]"
            value={notifyEvent}
            onChange={(e) => setNotifyEvent(e.target.value)}
            placeholder={t("settings.prefs.eventTypePlaceholder")}
          />
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={addLocalLogPolicy.isPending}
            onClick={() => addLocalLogPolicy.mutate()}
          >
            {t("settings.addLocalLog")}
          </button>
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={testNotify.isPending}
            onClick={() => testNotify.mutate()}
          >
            {t("settings.testNotify")}
          </button>
        </div>
        {testNotify.isSuccess && (
          <p className="text-sm text-secondary m-0 mt-2">{t("settings.notifySent")}</p>
        )}
      </div>

      {rows.length === 0 ? (
        <p className="text-sm text-secondary px-4 py-4 m-0">{t("settings.noPolicies")}</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="dw-table">
            <thead>
              <tr>
                <th>{t("settings.notifyEvent")}</th>
                <th>{t("settings.notifyChannel")}</th>
                <th>{t("common.status")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {rows.map((p) => (
                <tr key={p.id}>
                  <td className="font-code text-xs">{p.event_type}</td>
                  <td>{p.channel}</td>
                  <td>
                    <StatusBadge status={p.enabled ? "ok" : "cancelled"} />
                  </td>
                  <td className="text-right whitespace-nowrap">
                    <button
                      type="button"
                      className="dw-btn-ghost text-xs"
                      disabled={toggle.isPending}
                      onClick={() => toggle.mutate({ id: p.id, enabled: !p.enabled })}
                    >
                      {p.enabled ? t("common.disable") : t("common.enable")}
                    </button>
                    <button
                      type="button"
                      className="dw-btn-ghost text-xs text-error"
                      disabled={remove.isPending}
                      onClick={() => remove.mutate(p.id)}
                    >
                      {t("common.delete")}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </SectionCard>
  );
}
