import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { api } from "@/api/client";
import { DeliverableCompactShell } from "@/components/deliverables/DeliverableCompactShell";
import { DeliverableIframePreview } from "@/components/deliverables/DeliverableIframePreview";
import { DeliverablePanelShell } from "@/components/deliverables/DeliverablePanelShell";
import { useT } from "@/i18n/context";
import { inferPreviewPath, resolvePreviewUrl } from "@/lib/previewPath";

type SlideManifest = {
  slides?: Array<{ source_html?: string }>;
};

type Props = {
  path: string;
  title: string;
  projectId: string;
  previewPath?: string;
  variant?: "compact" | "full";
};

function deckDir(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const slash = normalized.lastIndexOf("/");
  if (slash <= 0) return ".";
  const base = normalized.slice(slash + 1).toLowerCase();
  if (base === "index.html" || base.endsWith(".pptx") || base.endsWith(".ppt")) {
    return normalized.slice(0, slash);
  }
  return normalized.slice(0, slash);
}

function manifestPathFor(path: string): string {
  return `${deckDir(path)}/slide_manifest.json`;
}

function slidePathFor(deckDirectory: string, sourceHtml: string): string {
  if (sourceHtml.includes("/")) return sourceHtml;
  return `${deckDirectory}/${sourceHtml}`;
}

export function PresentationThumbViewer({
  path,
  title,
  projectId,
  previewPath,
  variant = "compact",
}: Props) {
  const t = useT();
  const resolvedPreview = previewPath?.trim() || inferPreviewPath(path);
  const deckIndex =
    resolvedPreview && resolvedPreview.toLowerCase().endsWith("index.html")
      ? resolvedPreview
      : resolvedPreview ?? path;
  const previewUrl = resolvePreviewUrl(projectId, deckIndex, undefined, "self");
  const metaLabel = t("conversations.deliverable.presentation");
  const manifestPath = manifestPathFor(path);

  const manifest = useQuery({
    queryKey: ["slide-manifest", projectId, manifestPath],
    queryFn: async () => {
      const res = await api.readProjectFs(projectId, manifestPath, 256 * 1024);
      return JSON.parse(res.file.content ?? "{}") as SlideManifest;
    },
    enabled: Boolean(projectId && manifestPath),
    staleTime: 60_000,
    retry: false,
  });

  const slideUrls = useMemo(() => {
    const dir = deckDir(path);
    return (manifest.data?.slides ?? [])
      .map((slide) => slide.source_html?.trim())
      .filter(Boolean)
      .slice(0, 4)
      .map((source) =>
        resolvePreviewUrl(projectId, slidePathFor(dir, source!), undefined, "self"),
      );
  }, [manifest.data, path, projectId]);

  const thumbGrid =
    slideUrls.length >= 2 ? (
      <div className="deliverable-ppt-thumb-grid" aria-hidden>
        {slideUrls.map((url) => (
          <DeliverableIframePreview
            key={url}
            src={url}
            title=""
            size="thumb"
            className="deliverable-ppt-thumb-grid__cell"
          />
        ))}
      </div>
    ) : (
      <DeliverableIframePreview src={previewUrl} title={title} size="thumb" />
    );

  const dialogBody = <DeliverableIframePreview src={previewUrl} title={title} size="dialog" />;

  if (variant === "compact") {
    return (
      <DeliverableCompactShell
        path={path}
        projectId={projectId}
        title={title}
        metaLabel={metaLabel}
        cardClassName="deliverable-compact-card deliverable-compact-card--preview-html"
        thumb={<div className="deliverable-compact-card__thumb">{thumbGrid}</div>}
        dialogBody={dialogBody}
      />
    );
  }

  return (
    <DeliverablePanelShell path={path} projectId={projectId} title={title} metaLabel={metaLabel}>
      {dialogBody}
    </DeliverablePanelShell>
  );
}
