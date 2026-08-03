import hljs from "highlight.js/lib/core";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import yaml from "highlight.js/lib/languages/yaml";
import bash from "highlight.js/lib/languages/bash";
import python from "highlight.js/lib/languages/python";
import markdown from "highlight.js/lib/languages/markdown";
import { useMemo } from "react";
import { useProjectFsRead } from "../hooks/useProjectFileTree";
import { kindForPath } from "@/lib/artifactKind";
import { basename } from "@/lib/pathUtils";
import { selectDeliverableViewer } from "@/lib/selectDeliverableViewer";
import { useT } from "@/i18n/context";

hljs.registerLanguage("rust", rust);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("python", python);
hljs.registerLanguage("markdown", markdown);

type Props = {
  projectId: string;
  filePath: string | null;
};

function langFromMime(mime: string, path: string): string {
  if (mime.includes("rust")) return "rust";
  if (mime.includes("typescript")) return "typescript";
  if (mime.includes("javascript")) return "javascript";
  if (mime.includes("json")) return "json";
  if (mime.includes("yaml")) return "yaml";
  if (mime.includes("shell")) return "bash";
  if (mime.includes("python")) return "python";
  if (mime.includes("markdown")) return "markdown";
  const ext = path.split(".").pop()?.toLowerCase();
  if (ext === "rs") return "rust";
  if (ext === "ts" || ext === "tsx") return "typescript";
  if (ext === "js" || ext === "jsx") return "javascript";
  return "plaintext";
}

// Kinds rendered through the deliverable viewer (report/office cards etc).
const PREVIEW_KINDS = new Set([
  "mindmap",
  "report",
  "spreadsheet",
  "presentation",
  "document",
]);

// Binary kinds are previewed through the raw-file viewers (not read_file).
const RAW_PREVIEW_KINDS = new Set(["image", "video", "pdf", "audio", "media"]);

// Plain markdown/mdx keeps the raw text preview so the whole file is readable.
function isMarkdown(path: string | null): boolean {
  return Boolean(path && /\.mdx?$/i.test(path));
}

export function FilePreview({ projectId, filePath }: Props) {
  const t = useT();
  const fileKind = filePath ? kindForPath(filePath) : "file";
  const fileName = filePath ? basename(filePath) : "";

  // Binary / office / deliverable kinds are rendered through the dedicated
  // viewer (raw file, image/video/pdf viewers, or deliverable card), so the
  // text read is skipped for them (read_file rejects binary files).
  // Plain markdown keeps the raw text preview so the whole file is readable.
  const useViewer =
    RAW_PREVIEW_KINDS.has(fileKind) ||
    (PREVIEW_KINDS.has(fileKind) && !isMarkdown(filePath));

  const read = useProjectFsRead(projectId, useViewer ? null : filePath);

  const html = useMemo(() => {
    if (!read.data?.file) return "";
    const { content, mime_hint, path } = read.data.file;
    const lang = langFromMime(mime_hint, path);
    if (lang === "plaintext") return content;
    try {
      return hljs.highlight(content, { language: lang }).value;
    } catch {
      return content;
    }
  }, [read.data?.file]);

  if (!filePath) {
    return (
      <p className="text-xs text-secondary px-3 py-4 m-0 text-center">
        {t("workbench.selectFile")}
      </p>
    );
  }

  if (useViewer) {
    return (
      <div className="flex flex-col min-h-0 h-full border-t border-outline-variant/60 p-3 bg-white">
        {selectDeliverableViewer({
          path: filePath,
          title: fileName,
          projectId,
          variant: "full",
        })}
      </div>
    );
  }

  if (read.isPending) {
    return <p className="text-xs text-secondary px-3 py-2 m-0">{t("common.loading")}</p>;
  }

  if (read.error) {
    return (
      <p className="text-xs text-secondary px-3 py-2 m-0">{(read.error as Error).message}</p>
    );
  }

  const file = read.data!.file;

  return (
    <div className="flex flex-col min-h-0 h-full border-t border-outline-variant/60 bg-white">
      <div className="px-3 py-1.5 text-[10px] font-code text-secondary truncate border-b border-outline-variant/40 shrink-0">
        {file.path}
        {file.truncated ? ` · ${t("workbench.truncated")}` : ""}
      </div>
      <pre className="flex-1 min-h-0 overflow-auto m-0 p-3 text-[11px] font-code leading-relaxed bg-white">
        <code dangerouslySetInnerHTML={{ __html: html }} />
      </pre>
    </div>
  );
}