import { useQuery } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { api } from "@/api/client";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { CopyButton } from "@/components/ui/CopyButton";
import { useT } from "@/i18n/context";

export function ArtifactDetailPage() {
  const t = useT();
  const { artifactId } = useParams({ from: "/_shell/assets/$artifactId" });
  const detail = useQuery({
    queryKey: ["artifact", artifactId],
    queryFn: () => api.artifactDetail(artifactId),
  });

  const a = detail.data?.artifact;
  if (detail.isLoading) return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  if (!a) return <div className="dw-alert-error">{t("artifactDetail.notFound")}</div>;

  return (
    <>
      <PageHeader
        title={a.artifact.title}
        subtitle={`${a.artifact.path} · ${a.artifact.kind}`}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("assets.title"), to: "/assets" },
          { label: a.artifact.title },
        ]}
        actions={
          <>
            <CopyButton text={a.artifact.path} label={t("artifactDetail.copyPath")} />
            {a.artifact.project_id && a.artifact.session_id && (
              <CopyButton
                text={`# project=${a.artifact.project_id} session=${a.artifact.session_id}\n${a.artifact.path}`}
                label={t("artifactDetail.copyProvenance")}
              />
            )}
          </>
        }
      />
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SectionCard title={t("artifactDetail.evidence")}>
          <dl className="grid grid-cols-[minmax(4rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
            <dt className="text-secondary font-medium m-0">{t("conversations.trust")}</dt>
            <dd className="m-0">
              <StatusBadge status={a.artifact.trust_level} />
            </dd>
            {a.artifact.project_name && (
              <>
                <dt className="text-secondary font-medium m-0">{t("assets.project")}</dt>
                <dd className="m-0">
                  {a.artifact.project_id && (
                    <Link
                      to="/projects/$projectId"
                      params={{ projectId: a.artifact.project_id }}
                    >
                      {a.artifact.project_name}
                    </Link>
                  )}
                </dd>
              </>
            )}
            {a.artifact.session_id && (
              <>
                <dt className="text-secondary font-medium m-0">{t("audit.session")}</dt>
                <dd className="m-0">
                  <Link
                    to="/sessions/$sessionId"
                    params={{ sessionId: a.artifact.session_id }}
                    className="font-code text-xs"
                  >
                    {a.artifact.session_id}
                  </Link>
                </dd>
              </>
            )}
            {a.artifact.verified_by_gate_name && (
              <>
                <dt className="text-secondary font-medium m-0">{t("artifactDetail.verifyGate")}</dt>
                <dd className="m-0">{a.artifact.verified_by_gate_name}</dd>
              </>
            )}
          </dl>
        </SectionCard>
        <SectionCard title={t("artifactDetail.versions")} noPadding>
          {a.versions.length === 0 && (
            <p className="text-sm text-secondary px-4 py-4 m-0">{t("artifactDetail.notIndexed")}</p>
          )}
          {a.versions.length > 0 && (
            <div className="overflow-x-auto">
              <table className="dw-table">
                <thead>
                  <tr>
                    <th>{t("audit.time")}</th>
                    <th>{t("artifactDetail.hash")}</th>
                    <th>{t("artifactDetail.size")}</th>
                  </tr>
                </thead>
                <tbody>
                  {a.versions.map((v) => (
                    <tr key={v.id}>
                      <td className="text-secondary text-xs">{v.indexed_at}</td>
                      <td>
                        <code className="font-code">{v.hash.slice(0, 16)}…</code>
                      </td>
                      <td>{v.summary}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </SectionCard>
      </div>
      {a.links.length > 0 && (
        <SectionCard title={t("artifactDetail.externalLinks")}>
          <ul className="m-0 pl-5 text-sm space-y-1">
            {a.links.map((l) => (
              <li key={l.id}>
                {l.link_type}: {l.target_url ?? l.target_id}
              </li>
            ))}
          </ul>
        </SectionCard>
      )}
      {a.report_markdown && (
        <SectionCard title={t("reports.title")}>
          <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-[480px] whitespace-pre-wrap m-0">
            {a.report_markdown}
          </pre>
        </SectionCard>
      )}
    </>
  );
}
