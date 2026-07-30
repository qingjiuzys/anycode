import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { CloudTeamPeer } from "@/api/client/cloudA2a";
import type { HandoffKind } from "@/api/client/lan";
import { ModalOverlay } from "@/components/ui/ModalOverlay";
import { useConversationShell } from "@/context/ConversationShellContext";
import { useT } from "@/i18n/context";

type WizardStep = "pick" | "confirm";

type Props = {
  open: boolean;
  onClose: () => void;
  cloudPeer: CloudTeamPeer | null;
  kind: HandoffKind;
  initialProjectId?: string;
  initialSessionId?: string;
};

export function HandoffWizard({
  open,
  onClose,
  cloudPeer,
  kind,
  initialProjectId,
  initialSessionId,
}: Props) {
  const t = useT();
  const qc = useQueryClient();
  const { projectOptions, sidebarFilteredRows, projectId } = useConversationShell();
  const [wizardProjectId, setWizardProjectId] = useState("");
  const [wizardSessionId, setWizardSessionId] = useState("");
  const [wizardTargetProjectId, setWizardTargetProjectId] = useState("");
  const [wizardStep, setWizardStep] = useState<WizardStep>("pick");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sentId, setSentId] = useState<string | null>(null);

  const peerName = cloudPeer?.display_name || cloudPeer?.device_name;

  useEffect(() => {
    if (!open) {
      setWizardStep("pick");
      setError(null);
      setSentId(null);
      setWizardProjectId("");
      setWizardSessionId("");
      setWizardTargetProjectId("");
      return;
    }
    if (initialProjectId) {
      setWizardProjectId(initialProjectId);
    } else if (projectId) {
      setWizardProjectId(projectId);
    }
    if (initialSessionId) {
      setWizardSessionId(initialSessionId);
    } else if (kind === "session" && sidebarFilteredRows[0]) {
      setWizardSessionId(sidebarFilteredRows[0].id);
    }
  }, [
    open,
    kind,
    projectId,
    initialProjectId,
    initialSessionId,
    sidebarFilteredRows,
  ]);

  const submitHandoff = async () => {
    if (!cloudPeer) return;
    setBusy(true);
    setError(null);
    try {
      const resp = await api.requestCloudHandoff({
        recipient_device_id: cloudPeer.device_id,
        recipient_instance_id: cloudPeer.instance_id,
        kind,
        project_id: wizardProjectId || undefined,
        session_id: kind === "session" ? wizardSessionId || undefined : undefined,
        target_project_id: kind === "session" ? wizardTargetProjectId || undefined : undefined,
      });
      setSentId(resp.handoff?.id ?? null);
      setWizardStep("confirm");
      void qc.invalidateQueries({ queryKey: ["cloud"] });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModalOverlay
      open={open}
      onClose={onClose}
      labelledBy="handoff-wizard-title"
      className="w-full max-w-md"
      zIndex={320}
    >
      <div className="glass-modal rounded-2xl shadow-xl overflow-hidden">
        <div className="px-4 pt-4 pb-3 border-b border-outline-variant">
          <h2 id="handoff-wizard-title" className="text-base font-semibold m-0">
            {kind === "project" ? t("colleagues.projectHandoff") : t("colleagues.sessionHandoff")}
            {peerName ? ` → ${peerName}` : ""}
          </h2>
        </div>
        <div className="p-4 space-y-3">
          {wizardStep === "pick" ? (
            <>
              <label className="block text-sm">
                <span className="text-on-surface-variant">{t("colleagues.sourceProject")}</span>
                <select
                  className="dw-input w-full mt-1"
                  value={wizardProjectId}
                  onChange={(e) => setWizardProjectId(e.target.value)}
                >
                  <option value="">{t("colleagues.selectProject")}</option>
                  {projectOptions.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>
              {kind === "session" ? (
                <>
                  <label className="block text-sm">
                    <span className="text-on-surface-variant">{t("colleagues.sourceSession")}</span>
                    <select
                      className="dw-input w-full mt-1"
                      value={wizardSessionId}
                      onChange={(e) => setWizardSessionId(e.target.value)}
                    >
                      <option value="">{t("colleagues.selectSession")}</option>
                      {sidebarFilteredRows.map((s) => (
                        <option key={s.id} value={s.id}>
                          {s.title || s.id}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="block text-sm">
                    <span className="text-on-surface-variant">{t("colleagues.targetProject")}</span>
                    <select
                      className="dw-input w-full mt-1"
                      value={wizardTargetProjectId}
                      onChange={(e) => setWizardTargetProjectId(e.target.value)}
                    >
                      <option value="">{t("colleagues.newProjectOnReceive")}</option>
                      {projectOptions.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </label>
                </>
              ) : null}
              {error ? <p className="text-sm text-error m-0">{error}</p> : null}
              <div className="flex justify-end gap-2 pt-2">
                <button type="button" className="dw-btn dw-btn--ghost" onClick={onClose}>
                  {t("common.cancel")}
                </button>
                <button
                  type="button"
                  className="dw-btn dw-btn--primary"
                  disabled={busy || !wizardProjectId || (kind === "session" && !wizardSessionId)}
                  onClick={() => void submitHandoff()}
                >
                  {busy ? t("common.sending") : t("colleagues.sendRequest")}
                </button>
              </div>
            </>
          ) : (
            <div className="text-sm space-y-2">
              <p className="m-0">{t("colleagues.requestSent")}</p>
              {sentId ? (
                <p className="m-0 text-on-surface-variant font-mono text-xs">{sentId}</p>
              ) : null}
              <button type="button" className="dw-btn dw-btn--primary" onClick={onClose}>
                {t("common.close")}
              </button>
            </div>
          )}
        </div>
      </div>
    </ModalOverlay>
  );
}
