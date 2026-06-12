import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { useT } from "@/i18n/context";

type Props = {
  projectId: string;
  disabled?: boolean;
  maxHits?: number;
  compact?: boolean;
};

export function KnowledgeSearchPreview({
  projectId,
  disabled = false,
  maxHits = 3,
  compact = false,
}: Props) {
  const t = useT();
  const [searchQ, setSearchQ] = useState("");
  const [searchRun, setSearchRun] = useState("");

  const search = useQuery({
    queryKey: ["project-knowledge-search", projectId, searchRun],
    queryFn: () => api.searchProjectKnowledge(projectId, searchRun, maxHits),
    enabled: !disabled && searchRun.trim().length >= 2,
  });

  function runSearch() {
    const q = searchQ.trim();
    if (q.length >= 2) setSearchRun(q);
  }

  return (
    <div className={`flex flex-col gap-2 ${compact ? "" : "pt-2 border-t border-outline-variant/60"}`}>
      <span className="text-xs font-medium text-secondary">
        {t("projectDetail.config.knowledge.searchInModal")}
      </span>
      <div className="flex flex-wrap gap-2">
        <input
          className="dw-input flex-1 min-w-[10rem] text-sm"
          placeholder={t("projectDetail.knowledgeSearchPlaceholder")}
          value={searchQ}
          disabled={disabled}
          onChange={(e) => setSearchQ(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") runSearch();
          }}
        />
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          disabled={disabled || searchQ.trim().length < 2 || search.isFetching}
          onClick={runSearch}
        >
          {search.isFetching ? "…" : t("projectDetail.knowledgeSearch")}
        </button>
      </div>
      {disabled && (
        <p className="text-xs text-secondary m-0">{t("projectDetail.knowledgeNotConfigured")}</p>
      )}
      {searchRun && !search.isFetching && (search.data?.hits ?? []).length === 0 && !disabled && (
        <p className="text-sm text-secondary m-0">{t("projectDetail.knowledgeSearchEmpty")}</p>
      )}
      {(search.data?.hits ?? []).map((hit, i) => (
        <div
          key={`${hit.source_file}-${i}`}
          className="text-sm border border-outline-variant rounded-md p-2 bg-surface-container-low"
        >
          <div className="font-code text-xs text-secondary mb-1">
            {hit.source_file} ·{" "}
            {t("projectDetail.knowledgeSearchHit").replace("{score}", hit.score.toFixed(2))}
          </div>
          <div className="text-secondary line-clamp-2 whitespace-pre-wrap text-xs">{hit.snippet}</div>
        </div>
      ))}
    </div>
  );
}
