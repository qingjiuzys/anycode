import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

/** Cloud-only incoming handoff banner. */
export function IncomingHandoffBanner() {
  const t = useT();
  const qc = useQueryClient();
  const cloud = useAccountCloud();

  const cloudIncomingQuery = useQuery({
    queryKey: ["cloud", "a2a", "incoming"],
    queryFn: () => api.listCloudIncoming(),
    refetchInterval: 4000,
    enabled: cloud.cloudLinked,
  });

  const cloudReq = cloudIncomingQuery.data?.incoming?.[0];
  if (!cloudReq) return null;

  const approve = async () => {
    await api.approveCloudHandoff(cloudReq.id, {});
    void qc.invalidateQueries({ queryKey: ["cloud"] });
    void qc.invalidateQueries({ queryKey: ["projects"] });
  };

  const reject = async () => {
    await api.rejectCloudHandoff(cloudReq.id);
    void qc.invalidateQueries({ queryKey: ["cloud"] });
  };

  return (
    <div className="mx-3 mt-2 rounded-xl border border-primary/30 bg-primary/10 px-3 py-2 text-sm flex flex-wrap items-center gap-2">
      <span>
        {t("colleagues.incomingRequest")}: {cloudReq.sender_name} ·{" "}
        {cloudReq.kind === "project"
          ? t("colleagues.projectHandoff")
          : t("colleagues.sessionHandoff")}{" "}
        · {cloudReq.project_name ?? cloudReq.project_id ?? ""} · {t("colleagues.cloudHandoff")}
      </span>
      <div className="flex gap-2 ml-auto">
        <button type="button" className="dw-btn dw-btn--ghost dw-btn--sm" onClick={() => void reject()}>
          {t("common.reject")}
        </button>
        <button type="button" className="dw-btn dw-btn--primary dw-btn--sm" onClick={() => void approve()}>
          {t("common.approve")}
        </button>
      </div>
    </div>
  );
}
