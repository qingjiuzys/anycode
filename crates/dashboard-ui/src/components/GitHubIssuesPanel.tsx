import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function GitHubIssuesPanel({
  connectorId,
  connectorName,
  repo,
}: {
  connectorId: string;
  connectorName: string;
  repo: string;
}) {
  const t = useT();
  const issues = useQuery({
    queryKey: ["github-issues", connectorId],
    queryFn: () => api.githubIssues(connectorId),
    staleTime: 300_000,
    retry: 1,
  });

  const rows = issues.data?.issues ?? [];

  return (
    <SectionCard title={`GitHub · ${connectorName}`} className="mt-4">
      <p className="text-sm text-secondary m-0 mb-2">
        {repo} · {t("settings.githubReadOnly")}
      </p>
      {issues.isLoading && (
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      )}
      {issues.isError && (
        <p className="text-sm text-error m-0">{(issues.error as Error).message}</p>
      )}
      {!issues.isLoading && !issues.isError && rows.length === 0 && (
        <p className="text-sm text-secondary m-0">{t("settings.githubNoIssues")}</p>
      )}
      {rows.length > 0 && (
        <ul className="m-0 pl-0 list-none space-y-2">
          {rows.map((issue) => (
            <li key={issue.number} className="text-sm border-b border-outline-variant pb-2">
              <a href={issue.html_url} target="_blank" rel="noreferrer" className="font-medium">
                #{issue.number} {issue.title}
              </a>
              <div className="text-xs text-secondary mt-0.5">
                {issue.labels.join(", ")} · {issue.updated_at.slice(0, 10)}
              </div>
            </li>
          ))}
        </ul>
      )}
    </SectionCard>
  );
}
