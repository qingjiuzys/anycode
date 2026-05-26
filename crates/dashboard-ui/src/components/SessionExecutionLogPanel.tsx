import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function SessionExecutionLogPanel({ sessionId }: { sessionId: string }) {
  const t = useT();
  const [offset, setOffset] = useState(0);
  const limit = 100;

  const log = useQuery({
    queryKey: ["session-execution-log", sessionId, offset],
    queryFn: () => api.sessionExecutionLog(sessionId, { offset, limit }),
  });

  const data = log.data?.execution_log;
  const lines = data?.lines ?? [];

  return (
    <SectionCard title={t("session.executionLog")}>
      {log.isLoading && (
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      )}
      {log.isError && (
        <p className="text-sm text-error m-0">{(log.error as Error).message}</p>
      )}
      {!log.isLoading && lines.length === 0 && (
        <p className="text-sm text-secondary m-0">{t("session.executionLogEmpty")}</p>
      )}
      {lines.length > 0 && (
        <ul className="m-0 p-0 list-none space-y-2 font-code text-xs">
          {lines.map((line) => (
            <li
              key={`${line.line_no}-${line.raw.slice(0, 40)}`}
              className="border border-outline-variant rounded p-2 bg-surface-container-low"
            >
              <div className="flex flex-wrap items-center gap-2 text-secondary mb-1">
                <span>L{line.line_no}</span>
                {line.event_type && (
                  <span className="text-primary">{line.event_type}</span>
                )}
                {line.title && <span>{line.title}</span>}
              </div>
              {line.body ? (
                <pre className="m-0 whitespace-pre-wrap break-all text-on-surface">{line.body}</pre>
              ) : (
                <pre className="m-0 whitespace-pre-wrap break-all text-on-surface">{line.raw}</pre>
              )}
            </li>
          ))}
        </ul>
      )}
      {(offset > 0 || data?.has_more) && (
        <div className="flex items-center gap-2 mt-3">
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={offset === 0 || log.isFetching}
            onClick={() => setOffset((o) => Math.max(0, o - limit))}
          >
            <Icon name="chevron_left" size={16} />
            {t("session.executionLogPrev")}
          </button>
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={!data?.has_more || log.isFetching}
            onClick={() => setOffset(data?.next_offset ?? offset + limit)}
          >
            {t("session.executionLogNext")}
            <Icon name="chevron_right" size={16} />
          </button>
        </div>
      )}
    </SectionCard>
  );
}
