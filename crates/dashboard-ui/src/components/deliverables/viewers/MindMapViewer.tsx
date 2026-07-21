import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useRef, useState } from "react";
import { Transformer } from "markmap-lib";
import { Markmap } from "markmap-view";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { useT } from "@/i18n/context";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  projectId: string;
};

const transformer = new Transformer();

export function MindMapViewer({ path, title, projectId }: Props) {
  const t = useT();
  const fileName = path.split(/[/\\]/).pop() ?? path;
  const svgRef = useRef<SVGSVGElement | null>(null);
  const mmRef = useRef<Markmap | null>(null);
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

  useEffect(() => {
    if (!content.data || !svgRef.current) return;
    const { root } = transformer.transform(content.data);
    if (!mmRef.current) {
      mmRef.current = Markmap.create(svgRef.current, {
        autoFit: true,
        duration: 300,
      });
    }
    void mmRef.current.setData(root).then(() => {
      mmRef.current?.fit();
      setReady(true);
    });
  }, [content.data]);

  const exportSvg = useCallback(() => {
    const svg = svgRef.current;
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
  }, [fileName]);

  const exportPng = useCallback(async () => {
    const svg = svgRef.current;
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
  }, [fileName]);

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

  return (
    <div className="glass-panel rounded-xl border border-outline-variant/40 overflow-hidden">
      <div className="px-4 py-2 border-b border-outline-variant/30 flex items-center justify-between gap-2">
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
            <Icon name="description" size={14} className="inline mr-1" />
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
      <div className="relative bg-surface-container-lowest h-[min(420px,65vh)]">
        {content.isPending ? (
          <p className="m-0 p-4 text-sm text-secondary">{t("common.loading")}</p>
        ) : content.isError ? (
          <p className="m-0 p-4 text-sm text-error">{(content.error as Error).message}</p>
        ) : !content.data?.trim() ? (
          <p className="m-0 p-4 text-sm text-secondary">
            {t("conversations.deliverable.mindmapEmpty")}
          </p>
        ) : (
          <svg ref={svgRef} className="w-full h-full" role="img" aria-label={title} />
        )}
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
