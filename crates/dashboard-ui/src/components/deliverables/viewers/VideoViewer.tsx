import { DeliverableFileActions } from "@/components/deliverables/DeliverableFileActions";
import { projectFsRawUrl } from "@/lib/projectFsUrl";

type Props = {
  path: string;
  title: string;
  projectId: string;
  mime?: string;
};

export function VideoViewer({ path, title, projectId, mime }: Props) {
  const rawUrl = projectFsRawUrl(projectId, path);
  const fileName = path.split(/[/\\]/).pop() ?? path;

  return (
    <div className="glass-panel rounded-xl border border-outline-variant/40 overflow-hidden">
      <div className="px-4 py-2 border-b border-outline-variant/30">
        <p className="m-0 text-sm font-medium truncate">{title}</p>
      </div>
      <video controls preload="metadata" className="w-full max-h-[min(420px,60vh)] bg-black">
        <source src={rawUrl} {...(mime ? { type: mime } : {})} />
        <track kind="captions" />
      </video>
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
