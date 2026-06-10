import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { api } from "@/api/client";
import { streamGateExecute } from "@/api/gateStream";
import type { GateExecuteResult } from "@/api/types";
import { GateRunHistory } from "@/components/GateRunHistory";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useProjectViewPrefs } from "@/hooks/useProjectViewPrefs";
import { useT } from "@/i18n/context";

export function ProjectGateConfigPanel({ projectId }: { projectId: string }) {
  const t = useT();
  const queryClient = useQueryClient();
  const { prefs, update } = useProjectViewPrefs(projectId);
  const [lastResult, setLastResult] = useState<GateExecuteResult | null>(null);
  const [liveLines, setLiveLines] = useState<string[]>([]);
  const [runningPreset, setRunningPreset] = useState<string | null>(null);
  const [streamError, setStreamError] = useState<string | null>(null);
  const [customName, setCustomName] = useState("");
  const [customCommand, setCustomCommand] = useState("");
  const [customRequired, setCustomRequired] = useState(true);
  const abortRef = useRef<AbortController | null>(null);

  const presets = useQuery({
    queryKey: ["gate-presets", projectId],
    queryFn: () => api.gatePresets(projectId),
    staleTime: 60_000,
  });

  const rows = presets.data?.presets ?? [];

  function toggleAcceptance(presetId: string) {
    const set = new Set(prefs.acceptancePresetIds);
    if (set.has(presetId)) {
      set.delete(presetId);
    } else {
      set.add(presetId);
    }
    update({ acceptancePresetIds: [...set] });
  }

  async function runGate(body: {
    preset_id?: string;
    name?: string;
    command?: string;
    required: boolean;
  }) {
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    setRunningPreset(body.preset_id ?? "custom");
    setStreamError(null);
    setLiveLines([]);
    setLastResult(null);
    try {
      await streamGateExecute(projectId, body, (ev) => {
        if (ev.type === "line") {
          setLiveLines((prev) => [...prev, ev.line]);
        } else if (ev.type === "done") {
          setLastResult(ev.result);
        } else if (ev.type === "error") {
          setStreamError(ev.error);
        }
      }, ac.signal);
      void queryClient.invalidateQueries({ queryKey: ["gates", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["events", projectId] });
      void queryClient.invalidateQueries({ queryKey: ["gate-run-history", projectId] });
    } catch (e) {
      if ((e as Error).name !== "AbortError") {
        setStreamError((e as Error).message);
      }
    } finally {
      setRunningPreset(null);
    }
  }

  const outputText =
    liveLines.length > 0 ? liveLines.join("\n") : (lastResult?.output_excerpt ?? "");

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-secondary m-0">{t("projectDetail.config.gates.intro")}</p>
      <p className="text-xs text-secondary m-0">{t("projectDetail.config.gates.presetSource")}</p>

      {presets.isLoading && (
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      )}

      {!presets.isLoading && rows.length === 0 && (
        <div className="rounded-lg border border-outline-variant bg-surface-container-low p-3 text-sm text-secondary">
          {t("projectDetail.config.gates.noPresets")}
        </div>
      )}

      {rows.length > 0 && (
        <ul className="list-none m-0 p-0 space-y-2">
          {rows.map((p) => {
            const isAcceptance = prefs.acceptancePresetIds.includes(p.id);
            const required = isAcceptance;
            return (
              <li
                key={p.id}
                className="flex flex-col gap-2 rounded-lg border border-outline-variant p-3 bg-surface-container-low"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-medium text-sm">{p.name}</span>
                  <label className="text-xs text-secondary inline-flex items-center gap-1.5">
                    <input
                      type="checkbox"
                      checked={isAcceptance}
                      onChange={() => toggleAcceptance(p.id)}
                      disabled={runningPreset != null}
                      className="accent-primary"
                    />
                    {t("projectDetail.config.gates.markAcceptance")}
                  </label>
                </div>
                <code className="text-xs text-secondary font-code break-all">{p.command}</code>
                <button
                  type="button"
                  className="dw-btn-secondary text-sm self-start"
                  disabled={runningPreset != null}
                  onClick={() =>
                    runGate({
                      preset_id: p.id,
                      required,
                    })
                  }
                >
                  {runningPreset === p.id
                    ? t("projectDetail.gateRunning")
                    : t("projectDetail.config.gates.runPreset")}
                </button>
              </li>
            );
          })}
        </ul>
      )}

      <div className="pt-3 border-t border-outline-variant">
        <h3 className="text-sm font-medium m-0 mb-2">{t("projectDetail.config.gates.customTitle")}</h3>
        <p className="text-xs text-secondary m-0 mb-2">{t("projectDetail.config.gates.customHint")}</p>
        <div className="flex flex-col gap-2">
          <input
            className="dw-input text-sm"
            placeholder={t("projectDetail.config.gates.customNamePlaceholder")}
            value={customName}
            onChange={(e) => setCustomName(e.target.value)}
          />
          <input
            className="dw-input font-code text-sm"
            placeholder="cargo test --workspace"
            value={customCommand}
            onChange={(e) => setCustomCommand(e.target.value)}
          />
          <label className="text-sm text-secondary inline-flex items-center gap-2">
            <input
              type="checkbox"
              className="accent-primary"
              checked={customRequired}
              onChange={(e) => setCustomRequired(e.target.checked)}
            />
            <span>{t("projectDetail.gateRequired")}</span>
          </label>
          <button
            type="button"
            className="dw-btn-secondary text-sm self-start"
            disabled={
              runningPreset != null || !customName.trim() || !customCommand.trim()
            }
            onClick={() => {
              void runGate({
                name: customName.trim(),
                command: customCommand.trim(),
                required: customRequired,
              });
            }}
          >
            {runningPreset === "custom"
              ? t("projectDetail.gateRunning")
              : t("projectDetail.config.gates.runCustom")}
          </button>
        </div>
      </div>

      {streamError && <p className="text-sm text-error m-0">{streamError}</p>}
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
          <pre className="m-0 text-xs overflow-x-auto max-h-48 whitespace-pre-wrap font-code text-secondary">
            {outputText || t("projectDetail.gateNoOutput")}
          </pre>
        </div>
      )}
      <GateRunHistory projectId={projectId} />
    </div>
  );
}
