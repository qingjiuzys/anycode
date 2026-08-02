import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { CloudTeamPeer } from "@/api/client/cloudA2a";
import type { HandoffKind } from "@/api/client/lan";
import { ColleagueContextMenu } from "@/components/colleagues/ColleagueContextMenu";
import { ColleaguesGraph } from "@/components/colleagues/ColleaguesGraph";
import { HandoffWizard } from "@/components/colleagues/HandoffWizard";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { PageHeader } from "@/components/ui/PageHeader";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";
import { demoColleagues, type GraphPeer } from "@/lib/colleaguesGraph";
import { parseHandoffIntent } from "@/lib/handoffIntent";
import type { EmbeddedPageProps } from "@/lib/pageProps";

export function ColleaguesPage({ initialSearch }: EmbeddedPageProps = {}) {
  const t = useT();
  const cloud = useAccountCloud();
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [wizardKind, setWizardKind] = useState<HandoffKind>("project");
  const [demoNotice, setDemoNotice] = useState(false);
  const openMenuRef = useRef<(peerId: string, event: React.MouseEvent) => void>(() => {});
  const bindOpenMenu = useCallback(
    (open: (peerId: string, event: React.MouseEvent) => void) => {
      openMenuRef.current = open;
    },
    [],
  );

  const pendingIntent = useMemo(
    () => parseHandoffIntent(initialSearch),
    [initialSearch],
  );

  const cloudPeersQuery = useQuery({
    queryKey: ["cloud", "a2a", "peers"],
    queryFn: () => api.listTeamPeers(),
    refetchInterval: 8000,
    enabled: cloud.cloudLinked,
  });

  const teamReady = cloudPeersQuery.data?.team_ready ?? false;
  const teamGate = cloudPeersQuery.data?.gate;

  const realPeers = useMemo(
    () => (cloudPeersQuery.data?.peers ?? []).filter((p: CloudTeamPeer) => p.online),
    [cloudPeersQuery.data?.peers],
  );

  const usingDemo = teamReady && realPeers.length === 0;

  const graphPeers: GraphPeer[] = useMemo(() => {
    if (usingDemo) return demoColleagues();
    return realPeers.map((p: CloudTeamPeer) => ({
      id: p.instance_id,
      name: p.display_name?.trim() || p.device_name,
      subtitle: p.email,
    }));
  }, [usingDemo, realPeers]);

  const selectedCloudPeer: CloudTeamPeer | null = useMemo(() => {
    if (usingDemo || !selectedPeerId) return null;
    return realPeers.find((p: CloudTeamPeer) => p.instance_id === selectedPeerId) ?? null;
  }, [usingDemo, selectedPeerId, realPeers]);

  const selfName =
    cloud.sessionDisplayName?.trim() ||
    cloud.user?.display_name?.trim() ||
    t("colleagues.you");

  const openWizard = useCallback(
    (kind: HandoffKind, peerId: string) => {
      if (usingDemo || peerId.startsWith("demo_")) {
        setDemoNotice(true);
        window.setTimeout(() => setDemoNotice(false), 2800);
        return;
      }
      setSelectedPeerId(peerId);
      setWizardKind(kind);
      setWizardOpen(true);
    },
    [usingDemo],
  );

  useEffect(() => {
    if (!pendingIntent || !selectedCloudPeer || wizardOpen) return;
    setWizardKind(pendingIntent.kind);
    setWizardOpen(true);
  }, [pendingIntent, selectedCloudPeer, wizardOpen]);

  return (
    <CcPageShell
      header={
        <PageHeader title={t("colleagues.title")} subtitle={t("colleagues.pageSubtitle")} />
      }
    >
      <div className="dw-colleagues-page">
        {!cloud.cloudLinked ? (
          <div className="dw-colleagues-page__empty">
            <Icon name="cloud" size={28} className="text-on-surface-variant" />
            <p className="m-0 text-sm text-on-surface-variant">{t("colleagues.cloudNotLinked")}</p>
          </div>
        ) : !teamReady ? (
          <div className="dw-colleagues-page__empty">
            <Icon name="users" size={28} className="text-on-surface-variant" />
            <p className="m-0 text-sm text-on-surface-variant">
              {teamGate === "invite_required"
                ? t("colleagues.teamGateInvite")
                : t("colleagues.teamGateSetup")}
            </p>
          </div>
        ) : (
          <>
            {pendingIntent ? (
              <div className="mx-0 mb-3 px-3 py-2 rounded-lg border border-primary/30 bg-primary/10 text-sm">
                {t("colleagues.pendingHandoffHint")}
              </div>
            ) : null}
            {usingDemo ? (
              <div className="mx-0 mb-3 px-3 py-2 rounded-lg border border-outline-variant bg-surface-container-low text-sm text-on-surface-variant">
                {t("colleagues.demoBanner")}
              </div>
            ) : null}
            {demoNotice ? (
              <div className="mx-0 mb-3 px-3 py-2 rounded-lg border border-primary/30 bg-primary/10 text-sm">
                {t("colleagues.demoCannotHandoff")}
              </div>
            ) : null}
            <ColleagueContextMenu
              onProjectHandoff={(peerId) => openWizard("project", peerId)}
              onSessionHandoff={(peerId) => openWizard("session", peerId)}
              onReady={bindOpenMenu}
            >
              <div className="dw-colleagues-page__graph dw-colleagues-page__graph--full">
                <ColleaguesGraph
                  selfName={selfName}
                  peers={graphPeers}
                  selectedPeerId={selectedPeerId}
                  onPeerInteract={(peerId, event) => {
                    setSelectedPeerId(peerId);
                    openMenuRef.current(peerId, event);
                  }}
                />
              </div>
            </ColleagueContextMenu>
          </>
        )}
      </div>

      <HandoffWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        cloudPeer={selectedCloudPeer}
        kind={wizardKind}
        initialProjectId={pendingIntent?.projectId}
        initialSessionId={pendingIntent?.sessionId}
      />
    </CcPageShell>
  );
}
