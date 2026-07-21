import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  projectId: string;
};

export function PdfViewer({ path, title, projectId }: Props) {
  const rawUrl = projectFsRawUrl(projectId, path);
  const fileName = path.split(/[/\\]/).pop() ?? path;

  return (
    <div className="glass-panel rounded-xl border border-outline-variant/40 overflow-hidden">
      <div className="px-4 py-2 border-b border-outline-variant/30">
        <p className="m-0 text-sm font-medium truncate">{title}</p>
      </div>
      <iframe
        src={rawUrl}
        title={title}
        className="w-full h-[min(480px,65vh)] bg-surface-container-lowest border-0"
      />
      <div className="px-4 pb-3">
        <DeliverableFileActions
          path={path}
          downloadUrl={rawUrl}
          downloadName={fileName}
          compact
        />
      </div>
    </div>
  );
}
