import { Icon } from "@/components/Icon";
import { usePendingApprovalCounts } from "@/components/SecurityApprovalInbox";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useT } from "@/i18n/context";

export function ControlCenterButton() {
  const t = useT();
  const { open, openControlCenter } = useControlCenter();
  const { pendingTotal } = usePendingApprovalCounts();

  if (open) return null;

  return (
    <button
      type="button"
      className="dw-control-fab"
      onClick={() => openControlCenter("/settings")}
      title={t("controlCenter.open")}
      aria-label={t("controlCenter.open")}
    >
      <Icon name="tune" size={22} />
      {pendingTotal > 0 && (
        <span className="dw-control-fab-badge" aria-hidden>
          {pendingTotal > 99 ? "99+" : pendingTotal}
        </span>
      )}
    </button>
  );
}
