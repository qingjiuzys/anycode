import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { HandoffKind, LanPeer } from "@/api/client/lan";
import { ColleaguesGraph } from "@/components/colleagues/ColleaguesGraph";
import { HandoffWizard } from "@/components/colleagues/HandoffWizard";
import { Icon } from "@/components/Icon";
import { CcPageShell } from "@/components/ui/CcPageShell";
import { PageHeader } from "@/components/ui/PageHeader";
import { useT } from "@/i18n/context";
import type { EmbeddedPageProps } from "@/lib/pageProps";

export function ColleaguesPage(_props: EmbeddedPageProps = {}) {
  const t = useT();
  const [selectedPeerId, setSelectedPeerId] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [wizardKind, setWizardKind] = useState<HandoffKind>("project");

  const peersQuery = useQuery({
    queryKey: ["lan", "peers"],
    queryFn: () => api.listPeers(),
    refetchInterval: 5000,
  });

  const peers = peersQuery.data?.peers ?? [];
  const enabled = peersQuery.data?.enabled ?? false;
  const selfName = peersQuery.data?.display_name?.trim() || t("colleagues.you");

  const selectedPeer: LanPeer | null = useMemo(
    () => peers.find((p) => p.instance_id === selectedPeerId) ?? null,
    [peers, selectedPeerId],
  );

  const openWizard = (kind: HandoffKind) => {
    if (!selectedPeer) return;
    setWizardKind(kind);
    setWizardOpen(true);
  };

  return (
    <CcPageShell
      header={
        <PageHeader title={t("colleagues.title")} subtitle={t("colleagues.pageSubtitle")} />
      }
    >
      <div className="dw-colleagues-page">
        {!enabled ? (
          <div className="dw-colleagues-page__empty">
            <Icon name="group" size={28} className="text-on-surface-variant" />
            <p className="m-0 text-sm text-on-surface-variant">{t("colleagues.disabled")}</p>
          </div>
        ) : (
          <div className="dw-colleagues-page__layout">
            <div className="dw-colleagues-page__graph">
              <ColleaguesGraph
                selfName={selfName}
                peers={peers}
                selectedPeerId={selectedPeerId}
                onSelectPeer={setSelectedPeerId}
              />
            </div>
            <aside className="dw-colleagues-page__detail glass-panel">
              {selectedPeer ? (
                <>
                  <div className="dw-colleagues-page__detail-head">
                    <div className="dw-colleagues-page__avatar" aria-hidden>
                      {selectedPeer.device_name.slice(0, 1).toUpperCase()}
                    </div>
                    <div className="min-w-0">
                      <h3 className="text-base font-semibold m-0 truncate">
                        {selectedPeer.device_name}
                      </h3>
                      <p className="text-xs text-on-surface-variant m-0 mt-0.5 font-mono">
                        {selectedPeer.host}:{selectedPeer.lan_port}
                      </p>
                    </div>
                  </div>
                  <dl className="dw-colleagues-page__meta">
                    <div>
                      <dt>{t("colleagues.metaVersion")}</dt>
                      <dd>v{selectedPeer.version}</dd>
                    </div>
                    <div>
                      <dt>{t("colleagues.metaLastSeen")}</dt>
                      <dd>{new Date(selectedPeer.last_seen).toLocaleString()}</dd>
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
                    {t("colleagues.selectPeerHint")}
                  </p>
                  {peers.length === 0 ? (
                    <p className="text-sm text-on-surface-variant m-0 mt-4">{t("colleagues.empty")}</p>
                  ) : null}
                </div>
              )}
            </aside>
          </div>
        )}
      </div>

      <HandoffWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        peer={selectedPeer}
        kind={wizardKind}
      />
    </CcPageShell>
  );
}
