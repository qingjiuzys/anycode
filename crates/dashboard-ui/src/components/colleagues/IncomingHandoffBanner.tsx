import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { useAccountCloud } from "@/hooks/useAccountCloud";
import { useT } from "@/i18n/context";

export function IncomingHandoffBanner() {
  const t = useT();
  const qc = useQueryClient();
  const cloud = useAccountCloud();

  const lanIncomingQuery = useQuery({
    queryKey: ["lan", "incoming"],
    queryFn: () => api.listIncoming(),
    refetchInterval: 4000,
  });

  const cloudIncomingQuery = useQuery({
    queryKey: ["cloud", "a2a", "incoming"],
    queryFn: () => api.listCloudIncoming(),
    refetchInterval: 4000,
    enabled: cloud.cloudLinked,
  });

  const lanReq = lanIncomingQuery.data?.requests?.[0];
  const cloudReq = cloudIncomingQuery.data?.incoming?.[0];
  const req = lanReq ?? cloudReq;
  const isCloud = !lanReq && Boolean(cloudReq);

  if (!req) return null;

  const senderName = isCloud
    ? (cloudReq?.sender_name ?? "")
    : (lanReq?.sender.device_name ?? "");

  const approve = async () => {
    if (isCloud && cloudReq) {
      await api.approveCloudHandoff(cloudReq.id, {});
      void qc.invalidateQueries({ queryKey: ["cloud"] });
    } else if (lanReq) {
      await api.approveHandoff(lanReq.id, {});
      void qc.invalidateQueries({ queryKey: ["lan"] });
    }
    void qc.invalidateQueries({ queryKey: ["projects"] });
  };

  const reject = async () => {
    if (isCloud && cloudReq) {
      await api.rejectCloudHandoff(cloudReq.id);
      void qc.invalidateQueries({ queryKey: ["cloud"] });
    } else if (lanReq) {
      await api.rejectHandoff(lanReq.id);
      void qc.invalidateQueries({ queryKey: ["lan"] });
    }
  };

  const projectLabel = isCloud
    ? cloudReq?.project_name ?? cloudReq?.project_id ?? ""
    : lanReq?.project_name ?? lanReq?.project_id ?? "";

  return (
    <div className="mx-3 mt-2 rounded-xl border border-primary/30 bg-primary/10 px-3 py-2 text-sm flex flex-wrap items-center gap-2">
      <span>
        {t("colleagues.incomingRequest")}: {senderName} ·{" "}
        {req.kind === "project" ? t("colleagues.projectHandoff") : t("colleagues.sessionHandoff")}{" "}
        · {projectLabel}
        {isCloud ? ` · ${t("colleagues.tabCloud")}` : ""}
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
