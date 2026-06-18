import { Suspense, useCallback, useEffect } from "react";
import { createPortal } from "react-dom";
import { Icon } from "@/components/Icon";
import { FeatureNav } from "@/components/feature-nav/FeatureNav";
import { useControlCenter } from "@/context/ControlCenterContext";
import { EmbeddedControlCenterProvider } from "@/context/EmbeddedControlCenterContext";
import { useT } from "@/i18n/context";
import type { FeatureNavItem } from "@/lib/featureNav";
import { parseControlCenterPath } from "@/lib/controlCenterPaths";
import {
  AgentsPage,
  ArtifactDetailPage,
  AssetsPage,
  AuditPage,
  AutomationsPage,
  HomePage,
  OverviewPage,
  ProjectDetailPage,
  ProjectsPage,
  ReportsPage,
  ServicePage,
  SettingsPage,
  SkillDetailPage,
} from "@/routes/lazyPages";

function ControlCenterContent({ path }: { path: string }) {
  const t = useT();
  const parsed = parseControlCenterPath(path);

  return (
    <EmbeddedControlCenterProvider>
      <div className="dw-control-center-content dw-embedded-page">
        {parsed.view === "home" && <HomePage embedded />}
        {parsed.view === "overview" && <OverviewPage embedded />}
        {parsed.view === "projects" && <ProjectsPage embedded />}
        {parsed.view === "project" && (
          <ProjectDetailPage embedded projectId={parsed.projectId} />
        )}
        {parsed.view === "automations" && <AutomationsPage embedded />}
        {parsed.view === "assets" && (
          <AssetsPage embedded initialSearch={parsed.search} />
        )}
        {parsed.view === "artifact" && (
          <ArtifactDetailPage embedded artifactId={parsed.artifactId} />
        )}
        {parsed.view === "reports" && (
          <ReportsPage embedded initialSearch={parsed.search} />
        )}
        {parsed.view === "audit" && <AuditPage embedded />}
        {parsed.view === "account" && <ServicePage embedded />}
        {parsed.view === "agents" && <AgentsPage embedded />}
        {parsed.view === "skill" && (
          <SkillDetailPage embedded skillId={parsed.skillId} />
        )}
        {parsed.view === "settings" && (
          <SettingsPage embedded initialSearch={parsed.search} />
        )}
        {parsed.view === "unknown" && (
          <div className="p-6 text-secondary text-sm">{t("controlCenter.unknownSection")}</div>
        )}
      </div>
    </EmbeddedControlCenterProvider>
  );
}

export function ControlCenterOverlay() {
  const t = useT();
  const { open, activePath, closeControlCenter, setActivePath } = useControlCenter();

  const onSelect = useCallback(
    (item: FeatureNavItem) => {
      setActivePath(item.to);
    },
    [setActivePath],
  );

  useEffect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeControlCenter();
    };
    document.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener("keydown", onKey);
    };
  }, [open, closeControlCenter]);

  if (!open) return null;

  return createPortal(
    <div className="dw-control-center" role="dialog" aria-modal aria-label={t("controlCenter.title")}>
      <aside className="dw-control-center-nav glass-panel">
        <button
          type="button"
          className="dw-control-center-back"
          onClick={closeControlCenter}
        >
          <Icon name="chevron_left" size={18} />
          {t("common.back")}
        </button>
        <FeatureNav activePath={activePath} onSelect={onSelect} />
      </aside>
      <div className="dw-control-center-main">
        <Suspense
          fallback={
            <div className="p-6 text-secondary text-sm">{t("common.loading")}</div>
          }
        >
          <ControlCenterContent path={activePath} />
        </Suspense>
      </div>
    </div>,
    document.body,
  );
}
