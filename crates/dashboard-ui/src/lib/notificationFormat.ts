import { formatEventTypeLabel } from "@/lib/eventFormat";

export const NOTIFICATION_PRESET_EVENTS = [
  "session_report_generated",
  "project_report_generated",
  "gate_failed",
  "session_blocked",
  "blocked_threshold_exceeded",
] as const;

export type NotificationPresetEvent = (typeof NOTIFICATION_PRESET_EVENTS)[number];

function notificationEventFieldKey(eventType: string, field: "name" | "desc"): string {
  const id = eventType.trim().toLowerCase();
  return `settings.notificationEvents.${id}.${field}`;
}

export function formatNotificationEventLabel(
  eventType: string,
  t: (key: string) => string,
): string {
  const key = notificationEventFieldKey(eventType, "name");
  const label = t(key);
  if (label !== key) {
    return label;
  }
  return formatEventTypeLabel(eventType, t);
}

export function formatNotificationEventDesc(
  eventType: string,
  t: (key: string) => string,
): string | undefined {
  const key = notificationEventFieldKey(eventType, "desc");
  const desc = t(key);
  return desc !== key ? desc : undefined;
}

export function formatNotificationChannelLabel(
  channel: string,
  t: (key: string) => string,
): string {
  const id = channel.trim().toLowerCase();
  const key = `settings.notificationChannels.${id}`;
  const label = t(key);
  return label !== key ? label : channel;
}
