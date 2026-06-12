import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { KnowledgeSearchPreview } from "@/components/project/KnowledgeSearchPreview";
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
  const statsQ = useQuery({
    queryKey: ["project-knowledge-stats", projectId],
    queryFn: () => api.projectKnowledgeStats(projectId),
  });
  const pathsQ = useQuery({
    queryKey: ["project-knowledge", projectId],
    queryFn: () => api.projectKnowledgePaths(projectId),
  });

  const stats = statsQ.data?.stats;
  const pathCount = pathsQ.data?.paths.length ?? 0;
  const searchDisabled = (stats?.chunk_count ?? 0) === 0;

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
      {stats || pathCount > 0 ? (
        <p className="text-xs text-secondary m-0 mb-3">
          {t("projectDetail.knowledgeStats")
            .replace("{paths}", String(stats?.path_count ?? pathCount))
            .replace("{chunks}", String(stats?.chunk_count ?? 0))}
        </p>
      ) : (
        <div className="mb-3">
          <p className="text-xs text-secondary m-0 mb-2">{t("projectDetail.knowledgeNotConfigured")}</p>
          <ol className="text-xs text-secondary m-0 pl-4 flex flex-col gap-1">
            <li>{t("projectDetail.knowledgeSetupStep1")}</li>
            <li>{t("projectDetail.knowledgeSetupStep2")}</li>
          </ol>
        </div>
      )}

      <KnowledgeSearchPreview
        projectId={projectId}
        disabled={searchDisabled}
        maxHits={3}
        compact
      />
    </SectionCard>
  );
}
