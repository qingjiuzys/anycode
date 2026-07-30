import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Transformer } from "markmap-lib";
import { Markmap } from "markmap-view";
import { api } from "@/api/client";
import { DeliverablePreviewDialog } from "@/components/deliverables/DeliverablePreviewDialog";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useT } from "@/i18n/context";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  projectId: string;
  /** Conversation cards use compact + modal; workbench preview can open expanded. */
  variant?: "compact" | "full";
};

const transformer = new Transformer();

function MindMapCanvas({
  content,
  compact = false,
  title,
}: {
  content: string;
  compact?: boolean;
  title: string;
}) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  const mmRef = useRef<Markmap | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    if (!content || !svgRef.current) return;
    const { root } = transformer.transform(content);
    if (!mmRef.current) {
      mmRef.current = Markmap.create(svgRef.current, {
        autoFit: true,
        duration: compact ? 0 : 300,
        paddingX: compact ? 8 : 16,
      });
    }
    void mmRef.current.setData(root).then(() => {
      mmRef.current?.fit();
      setReady(true);
    });
  }, [content, compact]);

  return (
    <svg
      ref={svgRef}
      className={`w-full ${compact ? "h-[88px]" : "h-[min(420px,65vh)]"}`}
      role="img"
      aria-label={title}
      data-ready={ready ? "true" : "false"}
    />
  );
}

export function MindMapViewer({ path, title, projectId, variant = "full" }: Props) {
  const t = useT();
  const dialogTitleId = useId();
  const fileName = path.split(/[/\\]/).pop() ?? path;
  const [dialogOpen, setDialogOpen] = useState(false);
  const [ready, setReady] = useState(false);

  const content = useQuery({
    queryKey: ["deliverable-mindmap", projectId, path],
    queryFn: async () => {
      const res = await api.readProjectFs(projectId, path, 512 * 1024);
      return res.file.content ?? "";
    },
    enabled: Boolean(projectId && path),
    staleTime: 60_000,
  });

  const exportSvg = useCallback(() => {
    const svg = document.querySelector(
      `[data-mindmap-export="${dialogTitleId}"] svg[data-ready="true"]`,
    ) as SVGSVGElement | null;
    if (!svg) return;
    const clone = svg.cloneNode(true) as SVGSVGElement;
    clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
    const blob = new Blob([clone.outerHTML], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = fileName.replace(/\.md$/i, "") + ".svg";
    a.click();
    URL.revokeObjectURL(url);
  }, [dialogTitleId, fileName]);

  const exportPng = useCallback(async () => {
    const svg = document.querySelector(
      `[data-mindmap-export="${dialogTitleId}"] svg[data-ready="true"]`,
    ) as SVGSVGElement | null;
    if (!svg) return;
    const xml = new XMLSerializer().serializeToString(svg);
    const svgUrl = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(xml)}`;
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error("png export failed"));
      img.src = svgUrl;
    });
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(img.width, 800);
    canvas.height = Math.max(img.height, 600);
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0);
    const a = document.createElement("a");
    a.href = canvas.toDataURL("image/png");
    a.download = fileName.replace(/\.md$/i, "") + ".png";
    a.click();
  }, [dialogTitleId, fileName]);

  const exportMd = useCallback(() => {
    if (!content.data) return;
    const blob = new Blob([content.data], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = fileName.endsWith(".md") ? fileName : `${fileName}.md`;
    a.click();
    URL.revokeObjectURL(url);
  }, [content.data, fileName]);

  const body = content.isPending ? (
    <p className="m-0 p-4 text-sm text-secondary">{t("common.loading")}</p>
  ) : content.isError ? (
    <p className="m-0 p-4 text-sm text-error">{(content.error as Error).message}</p>
  ) : !content.data?.trim() ? (
    <p className="m-0 p-4 text-sm text-secondary">{t("conversations.deliverable.mindmapEmpty")}</p>
  ) : (
    <MindMapCanvas
      content={content.data}
      compact={variant === "compact" && !dialogOpen}
      title={title}
    />
  );

  useEffect(() => {
    if (content.data?.trim()) setReady(true);
  }, [content.data]);

  if (variant === "compact") {
    const mindmapLabel = t("conversations.deliverable.mindmapOutline");
    return (
      <>
        <button
          type="button"
          className="deliverable-compact-card deliverable-compact-card--mindmap"
          onClick={() => setDialogOpen(true)}
          aria-label={`${title}，${mindmapLabel}`}
        >
          <div className="deliverable-compact-card__thumb" aria-hidden>
            {content.isPending ? (
              <div className="deliverable-compact-card__thumb-placeholder" />
            ) : content.data?.trim() ? (
              <MindMapCanvas content={content.data} compact title={title} />
            ) : (
              <div className="deliverable-compact-card__thumb-placeholder" />
            )}
          </div>
          <div className="deliverable-compact-card__body min-w-0">
            <p className="deliverable-compact-card__title">{title}</p>
            <p className="deliverable-compact-card__meta">{mindmapLabel}</p>
          </div>
        </button>

        <DeliverablePreviewDialog
          open={dialogOpen}
          onClose={() => setDialogOpen(false)}
          title={title}
          subtitle={t("conversations.deliverable.mindmapOutline")}
          wide
        >
          <div data-mindmap-export={dialogTitleId}>
            {content.data?.trim() ? (
              <MindMapCanvas content={content.data} title={title} />
            ) : (
              body
            )}
          </div>
          <div className="px-4 py-3 border-t border-outline-variant/30 flex flex-wrap gap-1.5">
            <button
              type="button"
              className="dw-btn-ghost text-xs py-1 px-2"
              disabled={!ready}
              onClick={exportMd}
            >
              MD
            </button>
            <button
              type="button"
              className="dw-btn-ghost text-xs py-1 px-2"
              disabled={!ready}
              onClick={exportSvg}
            >
              SVG
            </button>
            <button
              type="button"
              className="dw-btn-ghost text-xs py-1 px-2"
              disabled={!ready}
              onClick={() => void exportPng()}
            >
              PNG
            </button>
            <DeliverableFileActions
              path={path}
              downloadUrl={projectFsRawUrl(projectId, path)}
              downloadName={fileName}
              compact
            />
          </div>
        </DeliverablePreviewDialog>
      </>
    );
  }

  return (
    <div className="deliverable-panel-card rounded-xl border border-outline-variant/30 overflow-hidden bg-white">
      <div className="px-4 py-2 border-b border-outline-variant/20 flex items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="m-0 text-sm font-medium truncate">{title}</p>
          <p className="m-0 mt-0.5 text-[11px] text-secondary">
            {t("conversations.deliverable.mindmapOutline")}
          </p>
        </div>
        <div className="flex flex-wrap gap-1 shrink-0">
          <button
            type="button"
            className="dw-btn-ghost text-xs py-1 px-2"
            disabled={!ready}
            onClick={exportMd}
          >
            MD
          </button>
          <button
            type="button"
            className="dw-btn-ghost text-xs py-1 px-2"
            disabled={!ready}
            onClick={exportSvg}
          >
            SVG
          </button>
          <button
            type="button"
            className="dw-btn-ghost text-xs py-1 px-2"
            disabled={!ready}
            onClick={() => void exportPng()}
          >
            PNG
          </button>
        </div>
      </div>
      <div className="relative bg-white" data-mindmap-export={dialogTitleId}>
        {body}
      </div>
      <div className="px-4 pb-3">
        <DeliverableFileActions
          path={path}
          downloadUrl={projectFsRawUrl(projectId, path)}
          downloadName={fileName}
          compact
        />
      </div>
    </div>
  );
}
