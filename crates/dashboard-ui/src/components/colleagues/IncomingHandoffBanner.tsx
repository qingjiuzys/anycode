import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { useT } from "@/i18n/context";

export function IncomingHandoffBanner() {
  const t = useT();
  const qc = useQueryClient();
  const incomingQuery = useQuery({
    queryKey: ["lan", "incoming"],
    queryFn: () => api.listIncoming(),
    refetchInterval: 4000,
  });

  const requests = incomingQuery.data?.requests ?? [];
  if (requests.length === 0) return null;

  const req = requests[0];

  const approve = async () => {
    await api.approveHandoff(req.id, {});
    void qc.invalidateQueries({ queryKey: ["lan"] });
    void qc.invalidateQueries({ queryKey: ["projects"] });
  };

  const reject = async () => {
    await api.rejectHandoff(req.id);
    void qc.invalidateQueries({ queryKey: ["lan"] });
  };

  return (
    <div className="mx-3 mt-2 rounded-xl border border-primary/30 bg-primary/10 px-3 py-2 text-sm flex flex-wrap items-center gap-2">
      <span>
        {t("colleagues.incomingRequest")}: {req.sender.device_name} ·{" "}
        {req.kind === "project" ? t("colleagues.projectHandoff") : t("colleagues.sessionHandoff")}{" "}
        · {req.project_name ?? req.project_id ?? ""}
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
