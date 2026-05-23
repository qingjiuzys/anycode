import { Link } from "@tanstack/react-router";
import { useAuth } from "@/auth/context";
import type { RuntimeSettings } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function AuthOrgPanel({ runtime }: { runtime?: RuntimeSettings }) {
  const t = useT();
  const { user } = useAuth();

  return (
    <SectionCard title={t("settings.authPanel.title")}>
      {user ? (
        <dl className="grid grid-cols-[minmax(5rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0 mb-4">
          <dt className="text-secondary font-medium m-0">{t("auth.email")}</dt>
          <dd className="m-0">{user.email}</dd>
          <dt className="text-secondary font-medium m-0">{t("common.name")}</dt>
          <dd className="m-0">{user.display_name}</dd>
          <dt className="text-secondary font-medium m-0">{t("auth.role")}</dt>
          <dd className="m-0">{user.role}</dd>
          <dt className="text-secondary font-medium m-0">{t("settings.runtime.authMode")}</dt>
          <dd className="m-0">
            <StatusBadge status={runtime?.auth_mode === "local_trusted" ? "ok" : "warn"} />
            <span className="ml-2 font-code text-xs">{runtime?.auth_mode ?? "…"}</span>
          </dd>
        </dl>
      ) : (
        <p className="text-sm text-secondary m-0 mb-4">{t("settings.authPanel.notSignedIn")}</p>
      )}
      <p className="text-sm text-secondary m-0 mb-2">{t("settings.authPanel.loopbackHint")}</p>
      <p className="text-sm text-secondary m-0 mb-2">{t("settings.authPanel.authModeDerived")}</p>
      <p className="text-sm text-secondary m-0">
        {runtime?.auth_mode === "token_required"
          ? t("settings.authPanel.remoteHint")
          : t("auth.localTrusted")}
      </p>
      {!user && (
        <Link to="/login" className="inline-block mt-3 text-sm">
          {t("auth.signIn")}
        </Link>
      )}
    </SectionCard>
  );
}
