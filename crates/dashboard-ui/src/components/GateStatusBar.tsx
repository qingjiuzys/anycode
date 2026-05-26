import type { GateRecord } from "@/api/types";
import { useT } from "@/i18n/context";
import { SessionStatusBadges, StatusBadge } from "./ui/StatusBadge";

interface Props {
  gates: GateRecord[];
  trustedStatus: string;
  sessionStatus: string;
}

const SEG_CLASS: Record<string, string> = {
  passed: "dw-gate-seg-ok",
  failed: "dw-gate-seg-fail",
  pending: "dw-gate-seg-pending",
  running: "dw-gate-seg-pending",
  skipped: "dw-gate-seg-pending",
};

export function GateStatusBar({ gates, trustedStatus, sessionStatus }: Props) {
  const t = useT();
  const required = gates.filter((g) => g.required);
  const passed = required.filter((g) => g.status === "passed").length;
  const failed = required.filter((g) => g.status === "failed").length;
  const pending = required.length - passed - failed;

  if (required.length === 0 && trustedStatus !== "blocked") {
    return null;
  }

  const progress = t("panels.gateProgress")
    .replace("{passed}", String(passed))
    .replace("{total}", String(required.length));

  const segmentWidth = required.length > 0 ? 100 / required.length : 100;

  return (
    <div className="dw-section-card p-4 flex flex-col gap-3">
      <div className="flex items-center justify-between text-sm">
        <span>
          {progress}
          {failed > 0 && (
            <span className="text-error ml-1">
              {t("panels.gateFailed").replace("{n}", String(failed))}
            </span>
          )}
          {pending > 0 && (
            <span className="text-secondary ml-1">
              {t("panels.gatePending").replace("{n}", String(pending))}
            </span>
          )}
        </span>
        <SessionStatusBadges
          status={sessionStatus}
          trustedStatus={trustedStatus}
        />
      </div>

      {required.length > 0 && (
        <div className="dw-gate-bar flex">
          {required.map((g) => (
            <div
              key={g.id}
              className={`${SEG_CLASS[g.status] ?? "dw-gate-seg-pending"} transition-all`}
              style={{ width: `${segmentWidth}%` }}
              title={`${g.name}: ${g.status}${g.output_excerpt ? ` — ${g.output_excerpt.slice(0, 120)}` : ""}`}
            />
          ))}
        </div>
      )}

      {required.length > 0 && (
        <div className="flex flex-wrap gap-x-4 gap-y-2">
          {required.map((g) => (
            <div key={g.id} className="text-xs min-w-[8rem]">
              <div className="flex items-center gap-1.5 font-medium text-on-surface">
                <span
                  className={`w-2 h-2 rounded-full shrink-0 ${
                    g.status === "passed"
                      ? "bg-success"
                      : g.status === "failed"
                        ? "bg-error"
                        : "bg-warn"
                  }`}
                />
                {g.name}
                <StatusBadge status={g.status} />
              </div>
              {g.output_excerpt && (
                <p className="text-secondary m-0 mt-0.5 line-clamp-2">{g.output_excerpt}</p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
