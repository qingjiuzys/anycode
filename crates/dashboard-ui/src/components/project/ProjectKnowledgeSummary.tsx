import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function ProjectKnowledgeSummary({
  projectId,
  onOpenConfig,
}: {
  projectId: string;
  onOpenConfig: () => void;
}) {
  const t = useT();
  const qc = useQueryClient();
  const statsQ = useQuery({
    queryKey: ["project-knowledge-stats", projectId],
    queryFn: () => api.projectKnowledgeStats(projectId),
  });
  const pathsQ = useQuery({
    queryKey: ["project-knowledge", projectId],
    queryFn: () => api.projectKnowledgePaths(projectId),
  });
  const [searchQ, setSearchQ] = useState("");
  const [searchRun, setSearchRun] = useState("");

  const search = useQuery({
    queryKey: ["project-knowledge-search", projectId, searchRun],
    queryFn: () => api.searchProjectKnowledge(projectId, searchRun),
    enabled: searchRun.trim().length >= 2,
  });

  const reindex = useMutation({
    mutationFn: () => api.reindexProjectKnowledge(projectId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["project-knowledge-stats", projectId] });
    },
  });

  const stats = statsQ.data?.stats;
  const pathCount = pathsQ.data?.paths.length ?? 0;

  return (
    <SectionCard
      title={t("projectDetail.knowledgeTitle")}
      action={
        <button type="button" className="dw-btn-secondary text-xs" onClick={onOpenConfig}>
          <Icon name="settings" size={14} />
          {t("projectDetail.config.open")}
        </button>
      }
    >
      <p className="text-sm text-secondary m-0 mb-2">{t("projectDetail.knowledgeSummaryHint")}</p>
      {stats ? (
        <p className="text-xs text-secondary m-0 mb-3">
          {t("projectDetail.knowledgeStats")
            .replace("{paths}", String(stats.path_count || pathCount))
            .replace("{chunks}", String(stats.chunk_count))}
        </p>
      ) : (
        <div className="mb-3">
          <p className="text-xs text-secondary m-0 mb-2">
            {pathCount > 0
              ? t("projectDetail.knowledgeStats")
                  .replace("{paths}", String(pathCount))
                  .replace("{chunks}", "0")
              : t("projectDetail.knowledgeNotConfigured")}
          </p>
          {pathCount === 0 && (
            <ol className="text-xs text-secondary m-0 pl-4 flex flex-col gap-1">
              <li>{t("projectDetail.knowledgeSetupStep1")}</li>
              <li>{t("projectDetail.knowledgeSetupStep2")}</li>
              <li>{t("projectDetail.knowledgeSetupStep3")}</li>
            </ol>
          )}
        </div>
      )}

      <div className="flex flex-wrap gap-2 mb-3">
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
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          disabled={reindex.isPending}
          onClick={() => reindex.mutate()}
        >
          {reindex.isPending ? t("projectDetail.reindexing") : t("projectDetail.knowledgeReindex")}
        </button>
      </div>

      {searchRun && !search.isFetching && (search.data?.hits ?? []).length === 0 && (
        <p className="text-sm text-secondary m-0">{t("projectDetail.knowledgeSearchEmpty")}</p>
      )}
      {(search.data?.hits ?? []).slice(0, 3).map((hit, i) => (
        <div
          key={`${hit.source_file}-${i}`}
          className="text-sm border border-outline-variant rounded-md p-2 bg-surface-container-low mb-2"
        >
          <div className="font-code text-xs text-secondary mb-1">
            {hit.source_file} ·{" "}
            {t("projectDetail.knowledgeSearchHit").replace("{score}", hit.score.toFixed(2))}
          </div>
          <div className="text-secondary line-clamp-3 whitespace-pre-wrap">{hit.snippet}</div>
        </div>
      ))}
    </SectionCard>
  );
}
