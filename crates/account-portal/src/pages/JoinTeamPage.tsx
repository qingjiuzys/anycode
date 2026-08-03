import { Navigate, useSearchParams } from "react-router-dom";
import { useAuth } from "../hooks/useAuth";
import { useT } from "../i18n/context";
import { SITE_PATHS } from "@anycode/site-urls";

/** Public entry for shared team invite links → login (if needed) → accept on Team page. */
export function JoinTeamPage() {
  const t = useT();
  const [params] = useSearchParams();
  const token = params.get("invite")?.trim() ?? "";
  const { authenticated, validating } = useAuth();

  if (!token) {
    return <Navigate to="/console/team" replace />;
  }

  const teamPath = `/console/team?invite=${encodeURIComponent(token)}`;

  if (validating) {
    return (
      <p className="muted console-meta" style={{ padding: "2rem" }}>
        {t("common.loading")}
      </p>
    );
  }

  if (!authenticated) {
    return <Navigate to={SITE_PATHS.login} replace state={{ from: teamPath }} />;
  }

  return <Navigate to={teamPath} replace />;
}
