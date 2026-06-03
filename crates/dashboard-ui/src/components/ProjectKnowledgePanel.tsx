import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

const DEFAULT_PATHS = ["docs/", "reports/", "references/"];

export function ProjectKnowledgePanel({ projectId }: { projectId: string }) {
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
  const [searchQ, setSearchQ] = useState("");
  const [searchRun, setSearchRun] = useState("");

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
    },
  });

  const reindex = useMutation({
    mutationFn: () => api.reindexProjectKnowledge(projectId),
    onSuccess: (data) => {
      void qc.invalidateQueries({ queryKey: ["project-knowledge", projectId] });
      void qc.invalidateQueries({ queryKey: ["project-knowledge-stats", projectId] });
      alert(t("projectDetail.knowledgeReindexed").replace("{n}", String(data.chunks_indexed)));
    },
  });

  const search = useQuery({
    queryKey: ["project-knowledge-search", projectId, searchRun],
    queryFn: () => api.searchProjectKnowledge(projectId, searchRun),
    enabled: searchRun.trim().length >= 2,
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
    <SectionCard title={t("projectDetail.knowledgeTitle")}>
      <p className="text-sm text-secondary m-0 mb-2">{t("projectDetail.knowledgeHint")}</p>
      {stats && (
        <p className="text-xs text-secondary m-0 mb-2">
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
        className="dw-input w-full min-h-[80px] font-code text-sm"
        placeholder="docs/\nreports/"
        value={display}
        onChange={(e) => setPathsText(e.target.value)}
      />
      <div className="flex flex-wrap gap-2 mt-2">
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
          {t("projectDetail.knowledgeReindex")}
        </button>
      </div>

      <div className="mt-4 pt-3 border-t border-outline-variant">
        <label className="block text-xs text-secondary mb-1">{t("projectDetail.knowledgeSearch")}</label>
        <div className="flex flex-wrap gap-2 mb-2">
          <input
            className="dw-input flex-1 min-w-[12rem] text-sm"
            placeholder={t("projectDetail.knowledgeSearchPlaceholder")}
            value={searchQ}
            onChange={(e) => setSearchQ(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && searchQ.trim().length >= 2) {
                setSearchRun(searchQ.trim());
              }
            }}
          />
          <button
            type="button"
            className="dw-btn-secondary text-sm"
            disabled={searchQ.trim().length < 2 || search.isFetching}
            onClick={() => setSearchRun(searchQ.trim())}
          >
            {search.isFetching ? "…" : t("projectDetail.knowledgeSearch")}
          </button>
        </div>
        {searchRun && !search.isFetching && (search.data?.hits ?? []).length === 0 && (
          <p className="text-sm text-secondary m-0">{t("projectDetail.knowledgeSearchEmpty")}</p>
        )}
        {(search.data?.hits ?? []).length > 0 && (
          <ul className="list-none m-0 p-0 space-y-2">
            {search.data?.hits.map((hit, i) => (
              <li
                key={`${hit.source_file}-${i}`}
                className="text-sm border border-outline-variant rounded-md p-2 bg-surface-container-low"
              >
                <div className="font-code text-xs text-secondary mb-1">
                  {hit.source_file} ·{" "}
                  {t("projectDetail.knowledgeSearchHit").replace("{score}", hit.score.toFixed(2))}
                </div>
                <div className="text-secondary whitespace-pre-wrap">{hit.snippet}</div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </SectionCard>
  );
}
