import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";
import { connectorPreviewErrorKey } from "@/lib/connectorErrors";

export function LinearIssuesPanel({
  connectorId,
  connectorName,
  team,
}: {
  connectorId: string;
  connectorName: string;
  team: string;
}) {
  const t = useT();
  const issues = useQuery({
    queryKey: ["linear-issues", connectorId],
    queryFn: () => api.linearIssues(connectorId),
    staleTime: 300_000,
    retry: 1,
  });

  const rows = issues.data?.issues ?? [];
  const rawError = issues.isError ? (issues.error as Error).message : "";
  const friendlyKey = rawError
    ? connectorPreviewErrorKey("linear", rawError)
    : null;

  return (
    <SectionCard title={`Linear · ${connectorName}`} className="mt-4">
      <p className="text-sm text-secondary m-0 mb-2">
        {team} · {t("settings.linearReadOnly")}
      </p>
      {issues.isLoading && (
        <p className="text-sm text-secondary m-0">{t("common.loading")}</p>
      )}
      {issues.isError && friendlyKey && (
        <div className="rounded-lg border border-error/30 bg-error/5 px-3 py-2" role="alert">
          <p className="text-sm text-error m-0">{t(friendlyKey)}</p>
          <details className="mt-1">
            <summary className="text-[12px] text-secondary cursor-pointer">
              {t("settings.connectorErrDetails")}
            </summary>
            <p className="text-[12px] text-secondary m-0 mt-1 break-all font-code">{rawError}</p>
          </details>
        </div>
      )}
      {!issues.isLoading && !issues.isError && rows.length === 0 && (
        <p className="text-sm text-secondary m-0">{t("settings.linearNoIssues")}</p>
      )}
      {rows.length > 0 && (
        <ul className="m-0 pl-0 list-none space-y-2">
          {rows.map((issue) => (
            <li key={issue.identifier} className="text-sm border-b border-outline-variant pb-2">
              <a href={issue.url} target="_blank" rel="noreferrer" className="font-medium">
                {issue.identifier} {issue.title}
              </a>
              <div className="text-xs text-secondary mt-0.5">
                {issue.state}
                {issue.labels.length > 0 ? ` · ${issue.labels.join(", ")}` : ""}
                {" · "}
                {issue.updated_at.slice(0, 10)}
              </div>
            </li>
          ))}
        </ul>
      )}
    </SectionCard>
  );
}
