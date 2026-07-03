import { useAuth } from "@/auth/context";
import { Icon } from "@/components/Icon";
import { NotificationsDropdown } from "@/components/NotificationsDropdown";
import { useControlCenter } from "@/context/ControlCenterContext";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function SidebarFooter() {
  const t = useT();
  const { user } = useAuth();
  const { openControlCenter } = useControlCenter();
  const { authenticated: cloudLinked, user: cloudUser, linkCloudAccount, linking } =
    useAccountCloud();

  const displayName =
    cloudLinked && cloudUser?.display_name
      ? cloudUser.display_name
      : user?.display_name || t("auth.localUser");
  const email =
    cloudLinked && cloudUser?.email ? cloudUser.email : user?.email || "local@anycode";
  const initials = displayName
    .split(/\s+/)
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();

  return (
    <footer className="dw-session-sidebar-footer">
      <button
        type="button"
        className="dw-session-sidebar-footer__profile w-full text-left border-0 bg-transparent p-0 cursor-pointer"
        title={cloudLinked ? t("nav.account") : t("service.cloud.linkAccount")}
        onClick={() => {
          if (cloudLinked) {
            openControlCenter("/account");
          } else {
            void linkCloudAccount().catch((err) => {
              console.error("linkCloudAccount:", err);
              openControlCenter("/account");
            });
          }
        }}
      >
        <span className="dw-session-sidebar-footer__avatar" aria-hidden>
          {initials || <Icon name="account_circle" size={20} />}
        </span>
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">
            {linking ? t("service.cloud.linking") : displayName}
          </div>
          <div className="text-[11px] text-secondary truncate">{email}</div>
        </div>
      </button>
      <div className="dw-session-sidebar-footer__actions">
        <NotificationsDropdown compact />
        <button
          type="button"
          className="dw-session-sidebar-footer__icon-btn"
          title={t("nav.settings")}
          aria-label={t("nav.settings")}
          onClick={() => openControlCenter("/settings")}
        >
          <Icon name="settings" size={18} />
        </button>
      </div>
    </footer>
  );
}
