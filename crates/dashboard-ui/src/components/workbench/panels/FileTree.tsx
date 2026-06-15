import { useState } from "react";
import type { FsEntry } from "@/api/types/workbench";
import { Icon } from "@/components/Icon";
import { useProjectFsList } from "../hooks/useProjectFileTree";

type Props = {
  projectId: string;
  selectedPath: string | null;
  onSelectPath: (path: string | null) => void;
};

type TreeNodeProps = {
  projectId: string;
  entry: FsEntry;
  depth: number;
  selectedPath: string | null;
  onSelectPath: (path: string | null) => void;
};

function TreeNode({ projectId, entry, depth, selectedPath, onSelectPath }: TreeNodeProps) {
  const [expanded, setExpanded] = useState(false);
  const isDir = entry.kind === "dir";
  const isSelected = selectedPath === entry.path;

  const children = useProjectFsList(projectId, expanded && isDir ? entry.path : "", {
    enabled: expanded && isDir,
  });
  const childEntries =
    expanded && isDir && children.data?.entries ? children.data.entries : [];

  return (
    <div>
      <button
        type="button"
        className={`conv-file-tree w-full flex items-center gap-1 py-0.5 pr-2 text-left text-xs border-0 bg-transparent cursor-pointer hover:bg-surface-container-low ${
          isSelected ? "bg-surface-container-high text-primary" : "text-on-surface"
        }`}
        style={{ paddingLeft: `${depth * 12 + 8}px` }}
        onClick={() => {
          if (isDir) {
            setExpanded((v) => !v);
          }
          onSelectPath(entry.path);
        }}
      >
        {isDir ? (
          <Icon name={expanded ? "expand_more" : "chevron_right"} size={14} className="shrink-0 text-secondary" />
        ) : (
          <span className="w-[14px] shrink-0" />
        )}
        <Icon
          name={isDir ? "folder" : "insert_drive_file"}
          size={14}
          className="shrink-0 text-secondary"
        />
        <span className="truncate">{entry.name}</span>
      </button>
      {expanded &&
        isDir &&
        childEntries.map((child) => (
          <TreeNode
            key={child.path}
            projectId={projectId}
            entry={child}
            depth={depth + 1}
            selectedPath={selectedPath}
            onSelectPath={onSelectPath}
          />
        ))}
    </div>
  );
}

export function FileTree({ projectId, selectedPath, onSelectPath }: Props) {
  const root = useProjectFsList(projectId, "");

  if (root.isPending) {
    return <p className="text-xs text-secondary px-3 py-2 m-0">Loading…</p>;
  }
  if (root.error) {
    return (
      <p className="text-xs text-error px-3 py-2 m-0">
        {(root.error as Error).message}
      </p>
    );
  }

  return (
    <div className="py-1 min-h-0 overflow-y-auto">
      {root.data?.entries.map((entry) => (
        <TreeNode
          key={entry.path}
          projectId={projectId}
          entry={entry}
          depth={0}
          selectedPath={selectedPath}
          onSelectPath={onSelectPath}
        />
      ))}
    </div>
  );
}
