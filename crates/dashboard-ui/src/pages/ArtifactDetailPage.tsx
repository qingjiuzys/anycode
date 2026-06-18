import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "@tanstack/react-router";
import { ControlCenterLink } from "@/components/control-center/ControlCenterLink";
import { api } from "@/api/client";
import { PageHeader } from "@/components/ui/PageHeader";
import { SectionCard } from "@/components/ui/SectionCard";
import { StatusBadge } from "@/components/ui/StatusBadge";
import { CopyButton } from "@/components/ui/CopyButton";
import { useT } from "@/i18n/context";
import { sessionChatSearch } from "@/lib/sessionLinks";
import type { EmbeddedPageProps } from "@/lib/pageProps";

export function ArtifactDetailPage({ embedded, artifactId: embeddedArtifactId }: EmbeddedPageProps = {}) {
  if (embedded) {
    if (!embeddedArtifactId) return <ArtifactDetailMissing />;
    return <ArtifactDetailInner artifactId={embeddedArtifactId} />;
  }
  return <ArtifactDetailRouted />;
}

function ArtifactDetailRouted() {
  const { artifactId } = useParams({ from: "/_shell/assets/$artifactId" });
  return <ArtifactDetailInner artifactId={artifactId} />;
}

function ArtifactDetailMissing() {
  const t = useT();
  return <div className="dw-alert-error">{t("artifactDetail.notFound")}</div>;
}

function ArtifactDetailInner({ artifactId }: { artifactId: string }) {
  const t = useT();
  const queryClient = useQueryClient();
  const detail = useQuery({
    queryKey: ["asset", artifactId],
    queryFn: () => api.assetDetail(artifactId),
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["asset", artifactId] });
    queryClient.invalidateQueries({ queryKey: ["assets"] });
  };

  const markReusable = useMutation({
    mutationFn: () => api.markAssetReusable(artifactId, {}),
    onSuccess: invalidate,
  });
  const archive = useMutation({
    mutationFn: () => api.archiveAsset(artifactId, {}),
    onSuccess: invalidate,
  });
  const promoteSkill = useMutation({
    mutationFn: () => api.promoteSkillDraft(artifactId),
    onSuccess: invalidate,
  });
  const promoteWorkflow = useMutation({
    mutationFn: () => api.promoteWorkflowDraft(artifactId),
    onSuccess: invalidate,
  });

  const a = detail.data?.asset;
  if (detail.isLoading) return <p className="text-sm text-secondary">{t("common.loading")}</p>;
  if (!a) return <div className="dw-alert-error">{t("artifactDetail.notFound")}</div>;

  const asset = a.asset;
  const artifact = a.artifact;
  const skill = a.skill;
  const isArtifact = asset.backend_type === "artifact";
  const canPromote = isArtifact && asset.asset_kind !== "skill" && asset.asset_kind !== "workflow";

  return (
    <>
      <PageHeader
        title={asset.title}
        subtitle={`${asset.subtitle} · ${t(`assets.kinds.${asset.asset_kind}`)}`}
        breadcrumbs={[
          { label: t("breadcrumb.home"), to: "/" },
          { label: t("assets.title"), to: "/assets" },
          { label: asset.title },
        ]}
        actions={
          <>
            {asset.path && (
              <CopyButton text={asset.path} label={t("artifactDetail.copyPath")} />
            )}
            {isArtifact && asset.project_id && asset.session_id && (
              <CopyButton
                text={`# project=${asset.project_id} session=${asset.session_id}\n${asset.path ?? ""}`}
                label={t("artifactDetail.copyProvenance")}
              />
            )}
          </>
        }
      />

      <div className="flex flex-wrap gap-2 mb-4">
        {isArtifact && asset.reuse_state !== "reusable" && (
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={markReusable.isPending}
            onClick={() => markReusable.mutate()}
          >
            {t("assets.actions.markReusable")}
          </button>
        )}
        {isArtifact && asset.reuse_state !== "archived" && (
          <button
            type="button"
            className="dw-btn-secondary"
            disabled={archive.isPending}
            onClick={() => archive.mutate()}
          >
            {t("assets.actions.archive")}
          </button>
        )}
        {canPromote && (
          <>
            <button
              type="button"
              className="dw-btn-secondary"
              disabled={promoteSkill.isPending}
              onClick={() => promoteSkill.mutate()}
            >
              {t("assets.actions.promoteSkill")}
            </button>
            <button
              type="button"
              className="dw-btn-secondary"
              disabled={promoteWorkflow.isPending}
              onClick={() => promoteWorkflow.mutate()}
            >
              {t("assets.actions.promoteWorkflow")}
            </button>
          </>
        )}
        {asset.backend_type === "skill" && (
          <Link to="/agents/$skillId" params={{ skillId: asset.backend_id }} className="dw-btn-secondary no-underline">
            {t("assets.actions.openSkill")}
          </Link>
        )}
      </div>

      {a.promotion_draft_path && (
        <div className="dw-alert-info mb-4 text-sm">
          {t("assets.draftCreated")}: <code className="font-code">{a.promotion_draft_path}</code>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <SectionCard title={t("artifactDetail.evidence")}>
          <dl className="grid grid-cols-[minmax(4rem,auto)_1fr] gap-x-4 gap-y-2 text-sm m-0">
            <dt className="text-secondary font-medium m-0">{t("assets.type")}</dt>
            <dd className="m-0">{t(`assets.kinds.${asset.asset_kind}`)}</dd>
            <dt className="text-secondary font-medium m-0">{t("assets.source")}</dt>
            <dd className="m-0">{t(`assets.sources.${asset.source_type}`)}</dd>
            <dt className="text-secondary font-medium m-0">{t("assets.reuseState")}</dt>
            <dd className="m-0">
              <StatusBadge status={asset.reuse_state} />
            </dd>
            <dt className="text-secondary font-medium m-0">{t("conversations.trust")}</dt>
            <dd className="m-0">
              <StatusBadge status={asset.trust_level} />
            </dd>
            {asset.project_name && (
              <>
                <dt className="text-secondary font-medium m-0">{t("assets.project")}</dt>
                <dd className="m-0">
                  {asset.project_id && (
                    <ControlCenterLink
                      to="/projects/$projectId"
                      params={{ projectId: asset.project_id }}
                    >
                      {asset.project_name}
                    </ControlCenterLink>
                  )}
                </dd>
              </>
            )}
            {asset.session_id && (
              <>
                <dt className="text-secondary font-medium m-0">{t("audit.session")}</dt>
                <dd className="m-0">
                  <Link
                    to="/conversations"
                    search={sessionChatSearch(asset.session_id, asset.project_id ?? undefined)}
                    className="font-code text-xs"
                  >
                    {asset.session_id}
                  </Link>
                </dd>
              </>
            )}
            {asset.verified_by_gate_name && (
              <>
                <dt className="text-secondary font-medium m-0">{t("artifactDetail.verifyGate")}</dt>
                <dd className="m-0">{asset.verified_by_gate_name}</dd>
              </>
            )}
            {asset.note && (
              <>
                <dt className="text-secondary font-medium m-0">{t("assets.note")}</dt>
                <dd className="m-0">{asset.note}</dd>
              </>
            )}
          </dl>
        </SectionCard>

        {skill && (
          <SectionCard title={t("assets.kinds.skill")}>
            <p className="text-sm m-0 mb-2">{skill.description}</p>
            <p className="text-xs text-secondary font-code m-0">{skill.source_path}</p>
            {skill.projects.length > 0 && (
              <ul className="text-sm mt-3 mb-0 pl-5">
                {skill.projects.filter((p) => p.enabled).map((p) => (
                  <li key={p.project_id}>{p.project_name}</li>
                ))}
              </ul>
            )}
          </SectionCard>
        )}

        {artifact && (
          <SectionCard title={t("artifactDetail.versions")} noPadding>
            {artifact.versions.length === 0 && (
              <p className="text-sm text-secondary px-4 py-4 m-0">{t("artifactDetail.notIndexed")}</p>
            )}
            {artifact.versions.length > 0 && (
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
                    {artifact.versions.map((v) => (
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
        )}
      </div>

      {artifact && artifact.links.length > 0 && (
        <SectionCard title={t("artifactDetail.externalLinks")}>
          <ul className="m-0 pl-5 text-sm space-y-1">
            {artifact.links.map((l) => (
              <li key={l.id}>
                {l.link_type}: {l.target_url ?? l.target_id}
              </li>
            ))}
          </ul>
        </SectionCard>
      )}

      {artifact?.report_markdown && (
        <SectionCard title={t("reports.title")}>
          <pre className="bg-surface-container-low border border-outline-variant rounded p-4 font-code text-xs overflow-auto max-h-[480px] whitespace-pre-wrap m-0">
            {artifact.report_markdown}
          </pre>
        </SectionCard>
      )}
    </>
  );
}
