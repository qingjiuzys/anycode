import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useRef, useState } from "react";
import { api } from "@/api/client";
import { streamGateExecute } from "@/api/gateStream";
import type { GateExecuteResult } from "@/api/types";
import { GateRunHistory } from "@/components/GateRunHistory";
import { Icon } from "@/components/Icon";
import { CollapsiblePanel } from "@/components/ui/CollapsiblePanel";
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

  const gates = useQuery({
    queryKey: ["gates", projectId],
    queryFn: () => api.gates(projectId),
    staleTime: 15_000,
  });

  const rows = presets.data?.presets ?? [];

  const lastStatusByName = useMemo(() => {
    const map = new Map<string, string>();
    for (const g of gates.data?.gates ?? []) {
      if (!map.has(g.name)) map.set(g.name, g.status);
    }
    return map;
  }, [gates.data?.gates]);

  function toggleAcceptance(presetId: string) {
    const set = new Set(prefs.acceptancePresetIds);
    if (set.has(presetId)) set.delete(presetId);
    else set.add(presetId);
    update({ acceptancePresetIds: [...set] });
  }

  function stopRun() {
    abortRef.current?.abort();
    setRunningPreset(null);
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

  const logTitle = lastResult
    ? `${lastResult.name} · ${lastResult.status} · ${lastResult.elapsed_ms}ms`
    : runningPreset
      ? t("projectDetail.gateStreaming")
      : t("projectDetail.config.gates.logTitle");

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
        <div className="overflow-x-auto rounded-lg border border-outline-variant">
          <table className="w-full text-sm border-collapse">
            <thead>
              <tr className="text-left text-xs text-secondary border-b border-outline-variant bg-surface-container-low">
                <th className="px-3 py-2 font-medium">{t("projectDetail.config.gates.colName")}</th>
                <th className="px-3 py-2 font-medium">{t("projectDetail.config.gates.colCommand")}</th>
                <th className="px-3 py-2 font-medium">{t("projectDetail.config.gates.colRequired")}</th>
                <th className="px-3 py-2 font-medium">{t("projectDetail.config.gates.colStatus")}</th>
                <th className="px-3 py-2 font-medium w-20">{t("projectDetail.config.gates.colAction")}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((p) => {
                const isAcceptance = prefs.acceptancePresetIds.includes(p.id);
                const last = lastStatusByName.get(p.name);
                return (
                  <tr key={p.id} className="border-b border-outline-variant/60 last:border-0">
                    <td className="px-3 py-2 font-medium align-top">{p.name}</td>
                    <td className="px-3 py-2 align-top">
                      <code className="text-xs font-code text-secondary break-all line-clamp-2" title={p.command}>
                        {p.command}
                      </code>
                    </td>
                    <td className="px-3 py-2 align-top">
                      <label
                        className="inline-flex items-center gap-2 cursor-pointer"
                        title={t("projectDetail.config.gates.markAcceptanceHint")}
                      >
                        <input
                          type="checkbox"
                          className="accent-primary w-4 h-4"
                          checked={isAcceptance}
                          onChange={() => toggleAcceptance(p.id)}
                          disabled={runningPreset != null}
                        />
                        <span className="text-xs text-secondary sr-only sm:not-sr-only">
                          {t("projectDetail.config.gates.markAcceptance")}
                        </span>
                      </label>
                    </td>
                    <td className="px-3 py-2 align-top">
                      {last ? (
                        <StatusBadge status={last === "passed" ? "passed" : "failed"} />
                      ) : (
                        <span className="text-xs text-secondary">
                          {t("projectDetail.config.gates.neverRun")}
                        </span>
                      )}
                    </td>
                    <td className="px-3 py-2 align-top">
                      {runningPreset === p.id ? (
                        <button
                          type="button"
                          className="dw-btn-secondary text-xs px-2 py-1"
                          onClick={stopRun}
                        >
                          {t("projectDetail.config.gates.stopRun")}
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="dw-btn-ghost p-1"
                          disabled={runningPreset != null}
                          title={t("projectDetail.config.gates.runPreset")}
                          onClick={() =>
                            runGate({ preset_id: p.id, required: isAcceptance })
                          }
                        >
                          <Icon name="play_arrow" size={20} />
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <CollapsiblePanel
        title={t("projectDetail.config.gates.addCustom")}
        defaultOpen={false}
        tone="muted"
        icon="add"
      >
        <div className="pt-2 flex flex-col gap-2">
          <p className="text-xs text-secondary m-0">{t("projectDetail.config.gates.customHint")}</p>
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
          <div className="flex gap-2">
            {runningPreset === "custom" ? (
              <button type="button" className="dw-btn-secondary text-sm" onClick={stopRun}>
                {t("projectDetail.config.gates.stopRun")}
              </button>
            ) : (
              <button
                type="button"
                className="dw-btn-secondary text-sm"
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
                {t("projectDetail.config.gates.runCustom")}
              </button>
            )}
          </div>
        </div>
      </CollapsiblePanel>

      {streamError && <p className="text-sm text-error m-0">{streamError}</p>}

      {(runningPreset || lastResult || liveLines.length > 0) && (
        <CollapsiblePanel
          title={logTitle}
          defaultOpen={Boolean(runningPreset)}
          tone={lastResult?.status === "failed" ? "error" : runningPreset ? "running" : "default"}
          icon="terminal"
        >
          <pre className="m-0 mt-2 text-xs overflow-x-auto max-h-48 whitespace-pre-wrap font-code text-secondary">
            {outputText || t("projectDetail.gateNoOutput")}
          </pre>
        </CollapsiblePanel>
      )}

      <GateRunHistory projectId={projectId} />
    </div>
  );
}
