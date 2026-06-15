import { useState } from "react";
import { FileTree } from "./FileTree";
import { FilePreview } from "./FilePreview";

type Props = {
  projectId: string;
};

export function FilesPanel({ projectId }: Props) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  return (
    <div className="flex flex-col h-full min-h-0">
      <div className="flex-1 min-h-[40%] max-h-[55%] overflow-hidden flex flex-col border-b border-outline-variant/60">
        <FileTree
          projectId={projectId}
          selectedPath={selectedPath}
          onSelectPath={setSelectedPath}
        />
      </div>
      <div className="flex-1 min-h-0">
        <FilePreview projectId={projectId} filePath={selectedPath} />
      </div>
    </div>
  );
}
