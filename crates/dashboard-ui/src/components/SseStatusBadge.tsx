import { useT } from "@/i18n/context";

export type SseStatus = "connecting" | "live" | "reconnecting" | "offline";

export function SseStatusBadge({ status }: { status: SseStatus }) {
  const t = useT();
  const labels: Record<SseStatus, string> = {
    connecting: t("layout.sseConnecting"),
    live: t("layout.sseLive"),
    reconnecting: t("layout.sseConnecting"),
    offline: t("layout.sseOffline"),
  };
  const dots: Record<SseStatus, string> = {
    connecting: "bg-warn",
    live: "bg-success",
    reconnecting: "bg-warn",
    offline: "bg-outline",
  };
  return (
    <span className="inline-flex items-center gap-1.5 text-xs text-secondary">
      <span className={`w-1.5 h-1.5 rounded-full ${dots[status]}`} />
      {labels[status]}
    </span>
  );
}
