import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { api } from "@/api/client";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

const MAX_VISIBLE = 4;

export function GateRunHistory({ projectId }: { projectId: string }) {
  const t = useT();
  const history = useQuery({
    queryKey: ["gate-run-history", projectId],
    queryFn: () => api.events(projectId, { eventType: "gate_executed", limit: 20 }),
    staleTime: 30_000,
  });

  const rows = history.data?.events ?? [];
  if (history.isLoading || rows.length === 0) return null;

  const visible = rows.slice(0, MAX_VISIBLE);

  return (
    <div className="mt-4 pt-3 border-t border-outline-variant/50">
      <div className="text-xs font-medium text-secondary mb-2">{t("projectDetail.gateHistory")}</div>
      <ul className="list-none m-0 p-0 space-y-2">
        {visible.map((ev) => {
          const payload = ev.payload ?? {};
          const name = String(payload.name ?? ev.title);
          const status = String(payload.status ?? "unknown");
          const elapsed = payload.elapsed_ms as number | undefined;
          return (
            <li
              key={ev.id}
              className="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg bg-surface-container-low px-3 py-2 text-xs"
            >
              <StatusBadge status={status === "passed" ? "passed" : "failed"} />
              <span className="font-medium text-on-surface">{name}</span>
              <span className="text-secondary tabular-nums">
                {ev.occurred_at.slice(0, 19).replace("T", " ")}
              </span>
              {elapsed != null && (
                <span className="text-secondary tabular-nums">{elapsed}ms</span>
              )}
              {ev.session_id && (
                <Link
                  to="/sessions/$sessionId"
                  params={{ sessionId: ev.session_id }}
                  className="text-primary no-underline hover:underline ml-auto"
                >
                  {t("projectDetail.viewSession")}
                </Link>
              )}
            </li>
          );
        })}
      </ul>
      {rows.length > MAX_VISIBLE && (
        <p className="text-xs text-secondary m-0 mt-2">
          {t("projectDetail.gateHistoryMore").replace("{n}", String(rows.length - MAX_VISIBLE))}
        </p>
      )}
    </div>
  );
}
