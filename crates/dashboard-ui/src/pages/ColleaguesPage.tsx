import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { CloudTeamPeer } from "@/api/client/cloudA2a";
import type { HandoffKind, LanPeer } from "@/api/client/lan";
import { ColleaguesGraph } from "@/components/colleagues/ColleaguesGraph";
import { HandoffWizard } from "@/components/colleagues/HandoffWizard";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { PageHeader } from "@/components/ui/PageHeader";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";
import { parseHandoffIntent } from "@/lib/handoffIntent";
import type { EmbeddedPageProps } from "@/lib/pageProps";

type PeerMode = "lan" | "cloud";

export function ColleaguesPage({ initialSearch }: EmbeddedPageProps = {}) {
  const t = useT();
  const cloud = useAccountCloud();
  const [peerMode, setPeerMode] = useState<PeerMode>("lan");
  const [selectedLanPeerId, setSelectedLanPeerId] = useState<string | null>(null);
  const [selectedCloudPeerId, setSelectedCloudPeerId] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [wizardKind, setWizardKind] = useState<HandoffKind>("project");

  const pendingIntent = useMemo(
    () => parseHandoffIntent(initialSearch),
    [initialSearch],
  );

  const peersQuery = useQuery({
    queryKey: ["lan", "peers"],
    queryFn: () => api.listPeers(),
    refetchInterval: 5000,
    enabled: peerMode === "lan",
  });

  const cloudPeersQuery = useQuery({
    queryKey: ["cloud", "a2a", "peers"],
    queryFn: () => api.listTeamPeers(),
    refetchInterval: 8000,
    enabled: peerMode === "cloud" && cloud.cloudLinked,
  });

  const lanPeers = peersQuery.data?.peers ?? [];
  const lanEnabled = peersQuery.data?.enabled ?? false;
  const selfName = peersQuery.data?.display_name?.trim() || t("colleagues.you");

  const cloudPeers = (cloudPeersQuery.data?.peers ?? []).filter(
    (p: CloudTeamPeer) => p.online,
  );

  const selectedLanPeer: LanPeer | null = useMemo(
    () => lanPeers.find((p) => p.instance_id === selectedLanPeerId) ?? null,
    [lanPeers, selectedLanPeerId],
  );

  const selectedCloudPeer: CloudTeamPeer | null = useMemo(
    () => cloudPeers.find((p: CloudTeamPeer) => p.instance_id === selectedCloudPeerId) ?? null,
    [cloudPeers, selectedCloudPeerId],
  );

  const openWizard = (kind: HandoffKind) => {
    if (peerMode === "lan" && !selectedLanPeer) return;
    if (peerMode === "cloud" && !selectedCloudPeer) return;
    setWizardKind(kind);
    setWizardOpen(true);
  };

  useEffect(() => {
    if (!pendingIntent) return;
    if (peerMode === "lan" && selectedLanPeer && !wizardOpen) {
      setWizardKind(pendingIntent.kind);
      setWizardOpen(true);
    }
    if (peerMode === "cloud" && selectedCloudPeer && !wizardOpen) {
      setWizardKind(pendingIntent.kind);
      setWizardOpen(true);
    }
  }, [pendingIntent, selectedLanPeer, selectedCloudPeer, peerMode, wizardOpen]);

  return (
    <CcPageShell
      header={
        <PageHeader title={t("colleagues.title")} subtitle={t("colleagues.pageSubtitle")} />
      }
    >
      <div className="flex gap-2 mb-3">
        <button
          type="button"
          className={`dw-btn dw-btn--sm ${peerMode === "lan" ? "dw-btn--primary" : "dw-btn--ghost"}`}
          onClick={() => setPeerMode("lan")}
        >
          {t("colleagues.tabLan")}
        </button>
        <button
          type="button"
          className={`dw-btn dw-btn--sm ${peerMode === "cloud" ? "dw-btn--primary" : "dw-btn--ghost"}`}
          onClick={() => setPeerMode("cloud")}
        >
          {t("colleagues.tabCloud")}
        </button>
      </div>

      <div className="dw-colleagues-page">
        {peerMode === "lan" && !lanEnabled ? (
          <div className="dw-colleagues-page__empty">
            <Icon name="group" size={28} className="text-on-surface-variant" />
            <p className="m-0 text-sm text-on-surface-variant">{t("colleagues.disabled")}</p>
          </div>
        ) : peerMode === "cloud" && !cloud.cloudLinked ? (
          <div className="dw-colleagues-page__empty">
            <Icon name="cloud" size={28} className="text-on-surface-variant" />
            <p className="m-0 text-sm text-on-surface-variant">{t("colleagues.cloudNotLinked")}</p>
          </div>
        ) : (
          <>
            {pendingIntent ? (
              <div className="mx-0 mb-3 px-3 py-2 rounded-lg border border-primary/30 bg-primary/10 text-sm">
                {t("colleagues.pendingHandoffHint")}
              </div>
            ) : null}
            <div className="dw-colleagues-page__layout">
              <div className="dw-colleagues-page__graph">
                {peerMode === "lan" ? (
                  <ColleaguesGraph
                    selfName={selfName}
                    peers={lanPeers}
                    selectedPeerId={selectedLanPeerId}
                    onSelectPeer={setSelectedLanPeerId}
                  />
                ) : (
                  <ul className="m-0 p-3 list-none space-y-2">
                    {cloudPeers.length === 0 ? (
                      <li className="text-sm text-on-surface-variant">{t("colleagues.cloudEmpty")}</li>
                    ) : (
                      cloudPeers.map((p: CloudTeamPeer) => (
                        <li key={p.instance_id}>
                          <button
                            type="button"
                            className={`w-full text-left px-3 py-2 rounded-lg border ${
                              selectedCloudPeerId === p.instance_id
                                ? "border-primary bg-primary/10"
                                : "border-outline-variant"
                            }`}
                            onClick={() => setSelectedCloudPeerId(p.instance_id)}
                          >
                            <div className="font-medium">{p.display_name || p.device_name}</div>
                            <div className="text-xs text-on-surface-variant font-mono">{p.email}</div>
                          </button>
                        </li>
                      ))
                    )}
                  </ul>
                )}
              </div>
              <aside className="dw-colleagues-page__detail glass-panel">
                {peerMode === "lan" && selectedLanPeer ? (
                  <>
                    <div className="dw-colleagues-page__detail-head">
                      <div className="dw-colleagues-page__avatar" aria-hidden>
                        {selectedLanPeer.device_name.slice(0, 1).toUpperCase()}
                      </div>
                      <div className="min-w-0">
                        <h3 className="text-base font-semibold m-0 truncate">
                          {selectedLanPeer.device_name}
                        </h3>
                        <p className="text-xs text-on-surface-variant m-0 mt-0.5 font-mono">
                          {selectedLanPeer.host}:{selectedLanPeer.lan_port}
                        </p>
                      </div>
                    </div>
                    <dl className="dw-colleagues-page__meta">
                      <div>
                        <dt>{t("colleagues.metaVersion")}</dt>
                        <dd>v{selectedLanPeer.version}</dd>
                      </div>
                      <div>
                        <dt>{t("colleagues.metaLastSeen")}</dt>
                        <dd>{new Date(selectedLanPeer.last_seen).toLocaleString()}</dd>
                      </div>
                    </dl>
                    <div className="dw-colleagues-page__actions">
                      <button
                        type="button"
                        className="dw-btn dw-btn--primary w-full"
                        onClick={() => openWizard("project")}
                      >
                        {t("colleagues.projectHandoff")}
                      </button>
                      <button
                        type="button"
                        className="dw-btn dw-btn--ghost w-full"
                        onClick={() => openWizard("session")}
                      >
                        {t("colleagues.sessionHandoff")}
                      </button>
                    </div>
                  </>
                ) : peerMode === "cloud" && selectedCloudPeer ? (
                  <>
                    <div className="dw-colleagues-page__detail-head">
                      <div className="dw-colleagues-page__avatar" aria-hidden>
                        {(selectedCloudPeer.display_name || selectedCloudPeer.device_name)
                          .slice(0, 1)
                          .toUpperCase()}
                      </div>
                      <div className="min-w-0">
                        <h3 className="text-base font-semibold m-0 truncate">
                          {selectedCloudPeer.display_name || selectedCloudPeer.device_name}
                        </h3>
                        <p className="text-xs text-on-surface-variant m-0 mt-0.5">
                          {t("colleagues.cloudHandoff")}
                        </p>
                      </div>
                    </div>
                    <dl className="dw-colleagues-page__meta">
                      <div>
                        <dt>{t("colleagues.metaVersion")}</dt>
                        <dd>v{selectedCloudPeer.version}</dd>
                      </div>
                      <div>
                        <dt>{t("colleagues.metaLastSeen")}</dt>
                        <dd>{new Date(selectedCloudPeer.last_seen).toLocaleString()}</dd>
                      </div>
                    </dl>
                    <div className="dw-colleagues-page__actions">
                      <button
                        type="button"
                        className="dw-btn dw-btn--primary w-full"
                        onClick={() => openWizard("project")}
                      >
                        {t("colleagues.projectHandoff")}
                      </button>
                      <button
                        type="button"
                        className="dw-btn dw-btn--ghost w-full"
                        onClick={() => openWizard("session")}
                      >
                        {t("colleagues.sessionHandoff")}
                      </button>
                    </div>
                  </>
                ) : (
                  <div className="dw-colleagues-page__detail-empty">
                    <p className="text-sm font-medium m-0">{selfName}</p>
                    <p className="text-xs text-on-surface-variant m-0 mt-1">
                      {pendingIntent
                        ? t("colleagues.pendingHandoffHint")
                        : t("colleagues.selectPeerHint")}
                    </p>
                  </div>
                )}
              </aside>
            </div>
          </>
        )}
      </div>

      <HandoffWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        transport={peerMode}
        lanPeer={selectedLanPeer}
        cloudPeer={selectedCloudPeer}
        kind={wizardKind}
        initialProjectId={pendingIntent?.projectId}
        initialSessionId={pendingIntent?.sessionId}
      />
    </CcPageShell>
  );
}
