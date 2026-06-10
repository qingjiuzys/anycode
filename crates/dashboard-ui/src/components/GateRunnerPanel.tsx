import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { api } from "@/api/client";
import { streamGateExecute } from "@/api/gateStream";
import type { GateExecuteResult } from "@/api/types";
import { GateRunHistory } from "@/components/GateRunHistory";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";

export function GateRunnerPanel({ projectId }: { projectId: string }) {
  const t = useT();
  const queryClient = useQueryClient();
  const [lastResult, setLastResult] = useState<GateExecuteResult | null>(null);
  const [liveLines, setLiveLines] = useState<string[]>([]);
  const [runningPreset, setRunningPreset] = useState<string | null>(null);
  const [streamError, setStreamError] = useState<string | null>(null);
  const [required, setRequired] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  const presets = useQuery({
    queryKey: ["gate-presets", projectId],
    queryFn: () => api.gatePresets(projectId),
    staleTime: 60_000,
  });

  const rows = presets.data?.presets ?? [];

  async function runPreset(presetId: string) {
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    setRunningPreset(presetId);
    setStreamError(null);
    setLiveLines([]);
    setLastResult(null);
    try {
      await streamGateExecute(
        projectId,
        { preset_id: presetId, required },
        (ev) => {
          if (ev.type === "line") {
            setLiveLines((prev) => [...prev, ev.line]);
          } else if (ev.type === "done") {
            setLastResult(ev.result);
          } else if (ev.type === "error") {
            setStreamError(ev.error);
          }
        },
        ac.signal,
      );
      queryClient.invalidateQueries({ queryKey: ["gates", projectId] });
      queryClient.invalidateQueries({ queryKey: ["events", projectId] });
      queryClient.invalidateQueries({ queryKey: ["gate-run-history", projectId] });
    } catch (e) {
      if ((e as Error).name !== "AbortError") {
        setStreamError((e as Error).message);
      }
    } finally {
      setRunningPreset(null);
    }
  }

  const outputText =
    liveLines.length > 0
      ? liveLines.join("\n")
      : lastResult?.output_excerpt ?? "";

  return (
    <SectionCard title={t("projectDetail.gateRunner")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("projectDetail.gateRunnerHint")}</p>
      <label className="text-sm text-secondary inline-flex items-center gap-2 mb-3">
        <input
          type="checkbox"
          checked={required}
          onChange={(e) => setRequired(e.target.checked)}
          disabled={runningPreset != null}
          className="accent-primary"
        />
        {t("projectDetail.gateRequired")}
      </label>
      {rows.length === 0 && (
        <p className="text-sm text-secondary m-0 mb-3">{t("projectDetail.config.gates.noPresets")}</p>
      )}
      <div className="flex flex-wrap gap-2 mb-3">
        {rows.map((p) => (
          <button
            key={p.id}
            type="button"
            className="dw-btn-secondary text-sm"
            disabled={runningPreset != null}
            onClick={() => runPreset(p.id)}
          >
            {runningPreset === p.id ? t("projectDetail.gateRunning") : p.name}
          </button>
        ))}
      </div>
      {streamError && <p className="text-sm text-error m-0 mb-2">{streamError}</p>}
      {(runningPreset || lastResult || liveLines.length > 0) && (
        <div className="border border-outline-variant rounded-lg p-3 text-sm">
          {lastResult && (
            <div className="flex items-center gap-2 mb-2">
              <StatusBadge status={lastResult.status === "passed" ? "passed" : "failed"} />
              <span className="font-medium">{lastResult.name}</span>
              {lastResult.elapsed_ms > 0 && (
                <span className="text-secondary text-xs">
                  {lastResult.elapsed_ms}ms · {lastResult.command}
                </span>
              )}
            </div>
          )}
          {runningPreset && !lastResult && (
            <p className="text-xs text-secondary m-0 mb-2">{t("projectDetail.gateStreaming")}</p>
          )}
          <pre className="m-0 text-xs overflow-x-auto max-h-64 whitespace-pre-wrap font-code text-secondary">
            {outputText || t("projectDetail.gateNoOutput")}
          </pre>
        </div>
      )}
      <GateRunHistory projectId={projectId} />
    </SectionCard>
  );
}
