import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { useT } from "@/i18n/context";

const DEFAULT_PATHS = ["docs/", "reports/", "references/"];

export function ProjectKnowledgeConfigPanel({ projectId }: { projectId: string }) {
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
  const [pathsText, setPathsText] = useState("");
  const [savedMsg, setSavedMsg] = useState<string | null>(null);

  useEffect(() => {
    if (!pathsText && pathsQ.data?.paths.length) {
      setPathsText(pathsQ.data.paths.join("\n"));
    }
  }, [pathsQ.data?.paths, pathsText]);

  const save = useMutation({
    mutationFn: (paths: string[]) => api.setProjectKnowledgePaths(projectId, paths),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["project-knowledge", projectId] });
      void qc.invalidateQueries({ queryKey: ["project-knowledge-stats", projectId] });
      setSavedMsg(t("projectDetail.config.knowledge.saved"));
      setTimeout(() => setSavedMsg(null), 3000);
    },
  });

  const reindex = useMutation({
    mutationFn: () => api.reindexProjectKnowledge(projectId),
    onSuccess: (data) => {
      void qc.invalidateQueries({ queryKey: ["project-knowledge", projectId] });
      void qc.invalidateQueries({ queryKey: ["project-knowledge-stats", projectId] });
      setSavedMsg(t("projectDetail.knowledgeReindexed").replace("{n}", String(data.chunks_indexed)));
      setTimeout(() => setSavedMsg(null), 5000);
    },
  });

  const display = pathsText || (pathsQ.data?.paths ?? []).join("\n");
  const stats = statsQ.data?.stats;

  function applyDefaultPaths() {
    const existing = new Set(
      display
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean),
    );
    for (const p of DEFAULT_PATHS) {
      existing.add(p);
    }
    setPathsText([...existing].join("\n"));
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-secondary m-0">{t("projectDetail.config.knowledge.intro")}</p>
      {stats && (
        <p className="text-xs text-secondary m-0">
          {t("projectDetail.knowledgeStats")
            .replace("{paths}", String(stats.path_count))
            .replace("{chunks}", String(stats.chunk_count))}
          {stats.cache_bytes != null
            ? ` · cache ${Math.round(stats.cache_bytes / 1024)} KB`
            : ""}
          {stats.vectors_enabled
            ? ` · vectors ${stats.vector_count ?? 0}`
            : ""}
        </p>
      )}
      <textarea
        className="dw-input w-full min-h-[100px] font-code text-sm"
        placeholder="docs/\nreports/"
        value={display}
        onChange={(e) => setPathsText(e.target.value)}
      />
      {savedMsg && <p className="text-sm text-primary m-0">{savedMsg}</p>}
      <div className="flex flex-wrap gap-2">
        <button type="button" className="dw-btn-secondary text-xs" onClick={applyDefaultPaths}>
          {t("projectDetail.knowledgeDefaults")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={save.isPending}
          onClick={() =>
            save.mutate(
              display
                .split("\n")
                .map((s) => s.trim())
                .filter(Boolean),
            )
          }
        >
          {t("projectDetail.knowledgeSave")}
        </button>
        <button
          type="button"
          className="dw-btn-secondary"
          disabled={reindex.isPending}
          onClick={() => reindex.mutate()}
        >
          {reindex.isPending ? t("projectDetail.reindexing") : t("projectDetail.knowledgeReindex")}
        </button>
      </div>
    </div>
  );
}
