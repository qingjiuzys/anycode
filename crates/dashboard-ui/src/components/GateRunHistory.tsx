import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function GateRunHistory({ projectId }: { projectId: string }) {
  const t = useT();
  const history = useQuery({
    queryKey: ["gate-run-history", projectId],
    queryFn: () => api.events(projectId, { eventType: "gate_executed", limit: 20 }),
    staleTime: 30_000,
  });

  const rows = history.data?.events ?? [];
  if (history.isLoading || rows.length === 0) return null;

  return (
    <div className="mt-4 border-t border-outline-variant pt-3">
      <div className="text-xs font-medium text-secondary mb-2">{t("projectDetail.gateHistory")}</div>
      <div className="overflow-x-auto">
        <table className="w-full text-xs border-collapse">
          <thead>
            <tr className="text-left text-secondary border-b border-outline/30">
              <th className="py-1.5 pr-3 font-medium">{t("common.time")}</th>
              <th className="py-1.5 pr-3 font-medium">{t("common.name")}</th>
              <th className="py-1.5 pr-3 font-medium">{t("common.status")}</th>
              <th className="py-1.5 pr-3 font-medium text-right">{t("projectDetail.gateElapsed")}</th>
              <th className="py-1.5 font-medium">{t("projectDetail.sessionLink")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((ev) => {
              const payload = ev.payload ?? {};
              const name = String(payload.name ?? ev.title);
              const status = String(payload.status ?? "unknown");
              const elapsed = payload.elapsed_ms as number | undefined;
              return (
                <tr key={ev.id} className="border-b border-outline/15 align-top">
                  <td className="py-1.5 pr-3 text-secondary whitespace-nowrap">
                    {ev.occurred_at.slice(0, 19).replace("T", " ")}
                  </td>
                  <td className="py-1.5 pr-3">
                    <div className="font-medium">{name}</div>
                    {ev.body && (
                      <pre className="m-0 mt-1 text-[10px] text-secondary max-h-16 overflow-auto whitespace-pre-wrap font-code">
                        {ev.body.slice(0, 400)}
                      </pre>
                    )}
                  </td>
                  <td className="py-1.5 pr-3">
                    <StatusBadge status={status === "passed" ? "passed" : "failed"} />
                  </td>
                  <td className="py-1.5 pr-3 text-right tabular-nums">
                    {elapsed != null ? `${elapsed}ms` : "—"}
                  </td>
                  <td className="py-1.5">
                    {ev.session_id ? (
                      <Link
                        to="/sessions/$sessionId"
                        params={{ sessionId: ev.session_id }}
                        className="text-primary no-underline hover:underline"
                      >
                        {t("projectDetail.viewSession")}
                      </Link>
                    ) : (
                      "—"
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
