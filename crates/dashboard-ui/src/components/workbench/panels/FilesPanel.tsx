import { useState } from "react";
import { FileTree } from "./FileTree";
import { FilePreview } from "./FilePreview";

type Props = {
  projectId: string;
};

/** Left file tree + right preview (horizontal split). */
export function FilesPanel({ projectId }: Props) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  return (
    <div className="flex h-full min-h-0 min-w-0">
      <div className="w-[min(40%,18rem)] shrink-0 overflow-hidden flex flex-col border-r border-outline-variant/60 bg-surface-container-low/40">
        <FileTree
          projectId={projectId}
          selectedPath={selectedPath}
          onSelectPath={setSelectedPath}
        />
      </div>
      <div className="flex-1 min-w-0 min-h-0">
        <FilePreview projectId={projectId} filePath={selectedPath} />
      </div>
    </div>
  );
}
