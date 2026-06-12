import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { KnowledgeSearchPreview } from "@/components/project/KnowledgeSearchPreview";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { useT } from "@/i18n/context";
import {
  DEFAULT_KNOWLEDGE_PATHS,
  knowledgeIndexStatus,
  pathsEqual,
  validateKnowledgePath,
} from "@/lib/knowledgePaths";

export function ProjectKnowledgeConfigPanel({
  projectId,
  onDirtyChange,
}: {
  projectId: string;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const t = useT();
  const qc = useQueryClient();
  const pathsQ = useQuery({
    queryKey: ["project-knowledge", projectId],
    queryFn: () => api.projectKnowledgePaths(projectId),
  });
  const statsQ = useQuery({
    queryKey: ["project-knowledge-stats", projectId],
    queryFn: () => api.projectKnowledgeStats(projectId),
  });

  const savedPaths = pathsQ.data?.paths ?? [];
  const [paths, setPaths] = useState<string[]>([]);
  const [draft, setDraft] = useState("");
  const [savedMsg, setSavedMsg] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    if (pathsQ.data) {
      setPaths(pathsQ.data.paths);
    }
  }, [pathsQ.data]);

  const dirty = !pathsEqual(paths, savedPaths);
  useEffect(() => {
    onDirtyChange?.(dirty);
  }, [dirty, onDirtyChange]);

  const stats = statsQ.data?.stats;
  const indexStatus = knowledgeIndexStatus(
    paths,
    savedPaths,
    stats?.chunk_count ?? 0,
  );

  const invalidPaths = useMemo(
    () => paths.filter((p) => validateKnowledgePath(p) != null),
    [paths],
  );

  const save = useMutation({
    mutationFn: (next: string[]) => api.setProjectKnowledgePaths(projectId, next),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["project-knowledge", projectId] });
      void qc.invalidateQueries({ queryKey: ["project-knowledge-stats", projectId] });
      setErrorMsg(null);
      setSavedMsg(t("projectDetail.config.knowledge.saved"));
      setTimeout(() => setSavedMsg(null), 3000);
    },
    onError: (e: Error) => {
      setErrorMsg(t("projectDetail.config.knowledge.saveError").replace("{msg}", e.message));
    },
  });

  const reindex = useMutation({
    mutationFn: () => api.reindexProjectKnowledge(projectId),
    onSuccess: (data) => {
      void qc.invalidateQueries({ queryKey: ["project-knowledge", projectId] });
      void qc.invalidateQueries({ queryKey: ["project-knowledge-stats", projectId] });
      setErrorMsg(null);
      setSavedMsg(t("projectDetail.knowledgeReindexed").replace("{n}", String(data.chunks_indexed)));
      setTimeout(() => setSavedMsg(null), 5000);
    },
    onError: (e: Error) => {
      setErrorMsg(t("projectDetail.config.knowledge.reindexError").replace("{msg}", e.message));
    },
  });

  function addPath(raw: string) {
    const trimmed = raw.trim();
    if (!trimmed || paths.includes(trimmed)) return;
    setPaths((prev) => [...prev, trimmed]);
    setDraft("");
  }

  function applyDefaultPaths() {
    setPaths((prev) => {
      const set = new Set(prev);
      for (const p of DEFAULT_KNOWLEDGE_PATHS) set.add(p);
      return [...set];
    });
  }

  function statusLabel() {
    if (indexStatus === "empty") return t("projectDetail.config.knowledge.statusEmpty");
    if (indexStatus === "stale") return t("projectDetail.config.knowledge.statusStale");
    return t("projectDetail.config.knowledge.statusReady");
  }

  function statusTone(): "pending" | "passed" {
    if (indexStatus === "ready") return "passed";
    return "pending";
  }

  const searchDisabled = indexStatus !== "ready";

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-secondary m-0">{t("projectDetail.config.knowledge.intro")}</p>

      <div className="rounded-lg border border-outline-variant bg-surface-container-low px-4 py-3 flex flex-wrap items-center justify-between gap-3">
        <div className="text-xs text-secondary space-y-1">
          <p className="m-0">
            {t("projectDetail.knowledgeStats")
              .replace("{paths}", String(paths.length))
              .replace("{chunks}", String(stats?.chunk_count ?? 0))}
          </p>
          {(stats?.cache_bytes != null || stats?.vectors_enabled) && (
            <p className="m-0">
              {stats?.cache_bytes != null
                ? t("projectDetail.config.knowledge.cacheKb").replace(
                    "{n}",
                    String(Math.round(stats.cache_bytes / 1024)),
                  )
                : null}
              {stats?.cache_bytes != null && stats?.vectors_enabled ? " · " : null}
              {stats?.vectors_enabled
                ? t("projectDetail.config.knowledge.vectorsCount").replace(
                    "{n}",
                    String(stats.vector_count ?? 0),
                  )
                : null}
            </p>
          )}
        </div>
        <StatusBadge status={statusTone()} label={statusLabel()} />
      </div>

      <div className="flex flex-col gap-2">
        <div className="flex flex-wrap gap-2 min-h-[2rem]">
          {paths.map((p) => (
            <span
              key={p}
              className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-xs font-code ${
                validateKnowledgePath(p)
                  ? "border-error/40 text-error bg-error-container/10"
                  : "border-outline-variant bg-surface-container-high"
              }`}
            >
              {p}
              <button
                type="button"
                className="dw-btn-ghost p-0.5 rounded-full"
                aria-label={t("common.delete")}
                onClick={() => setPaths((prev) => prev.filter((x) => x !== p))}
              >
                <Icon name="close" size={14} />
              </button>
            </span>
          ))}
        </div>
        {invalidPaths.length > 0 && (
          <p className="text-xs text-error m-0">{t("projectDetail.config.knowledge.pathInvalid")}</p>
        )}
        <div className="flex flex-wrap gap-2">
          <input
            className="dw-input flex-1 min-w-[12rem] text-sm font-code"
            placeholder={t("projectDetail.config.knowledge.pathPlaceholder")}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                addPath(draft);
              }
            }}
          />
          <button type="button" className="dw-btn-secondary text-xs" onClick={applyDefaultPaths}>
            {t("projectDetail.knowledgeDefaults")}
          </button>
        </div>
      </div>

      {savedMsg && <p className="text-sm text-primary m-0">{savedMsg}</p>}
      {errorMsg && <p className="text-sm text-error m-0">{errorMsg}</p>}

      <div className="flex flex-wrap justify-end gap-2">
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={save.isPending || !dirty || invalidPaths.length > 0}
          onClick={() => save.mutate(paths)}
        >
          {t("projectDetail.knowledgeSave")}
        </button>
        <button
          type="button"
          className="dw-btn-primary"
          disabled={reindex.isPending || paths.length === 0 || invalidPaths.length > 0 || dirty}
          onClick={() => reindex.mutate()}
        >
          {reindex.isPending ? t("projectDetail.reindexing") : t("projectDetail.knowledgeReindex")}
        </button>
      </div>

      {paths.length === 0 && (
        <ol className="text-xs text-secondary m-0 pl-4 flex flex-col gap-1">
          <li>{t("projectDetail.knowledgeSetupStep1")}</li>
          <li>{t("projectDetail.knowledgeSetupStep2")}</li>
        </ol>
      )}

      <KnowledgeSearchPreview projectId={projectId} disabled={searchDisabled} maxHits={3} />
    </div>
  );
}
