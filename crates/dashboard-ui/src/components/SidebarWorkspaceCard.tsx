import { useAuth } from "@/auth/context";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";

export function SidebarWorkspaceCard() {
  const t = useT();
  const { user } = useAuth();

  return (
    <div className="mx-2 mb-3 p-3 rounded-lg bg-surface-container-low border border-outline-variant">
      <div className="text-[10px] uppercase font-semibold text-secondary tracking-wide mb-1">
        {t("layout.workspace")}
      </div>
      <div className="flex items-center gap-2">
        <div className="w-8 h-8 rounded-full bg-primary/15 text-primary flex items-center justify-center shrink-0">
          <Icon name="corporate_fare" size={18} />
        </div>
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">
            {user?.display_name || t("auth.localUser")}
          </div>
          <div className="text-xs text-secondary truncate">{user?.email || "local@anycode"}</div>
        </div>
      </div>
    </div>
  );
}
