import { useMemo, useCallback, type CSSProperties } from "react";
import { useNavigate } from "@tanstack/react-router";
import ReactFlow, {
  Background,
  Controls,
  type Edge,
  type Node,
  type NodeMouseHandler,
} from "reactflow";
import "reactflow/dist/style.css";
import type { SessionSummary } from "@/api/types";
import { formatSessionDisplayTitle, formatSessionFlowStatusLine, isImportedSessionTitle } from "@/lib/eventFormat";
import { useT } from "@/i18n/context";
import { sessionChatSearch } from "@/lib/sessionLinks";

interface Props {
  sessions: SessionSummary[];
  limit?: number;
  hideImported?: boolean;
  /** Read-only compact preview for settings modal */
  preview?: boolean;
}

const KIND_COLORS: Record<string, string> = {
  run: "#2563eb",
  goal: "#7c3aed",
  workflow: "#0891b2",
  repl: "#4f46e5",
  cron: "#ca8a04",
};

export function SessionFlow({ sessions, limit = 8, hideImported = false, preview = false }: Props) {
  const t = useT();
  const navigate = useNavigate();
  const { nodes, edges } = useMemo(() => {
    const nodes: Node[] = [
      {
        id: "start",
        position: { x: 0, y: 80 },
        data: { label: t("sessionFlow.start") },
        style: nodeStyle("#2563eb"),
      },
    ];
    const edges: Edge[] = [];
    let filtered = [...sessions];
    if (hideImported) {
      filtered = filtered.filter((s) => !isImportedSessionTitle(s.title));
    }
    const ordered = filtered
      .sort((a, b) => a.started_at.localeCompare(b.started_at))
      .slice(-limit);
    ordered.forEach((s, i) => {
      const id = s.id;
      const border = sessionBorderColor(s);
      const titleLine = formatSessionDisplayTitle(s.title, s.kind, t);
      nodes.push({
        id,
        position: { x: 220 + i * 180, y: 40 + (i % 2) * 80 },
        data: {
          label: `${titleLine}\n${formatSessionFlowStatusLine(s.status, s.trusted_status, t)}`,
        },
        style: nodeStyle(border),
      });
      const prev = i === 0 ? "start" : ordered[i - 1].id;
      edges.push({
        id: `e-${prev}-${id}`,
        source: prev,
        target: id,
        style: s.trusted_status === "blocked" ? { stroke: "#ba1a1a" } : undefined,
      });
    });
    return { nodes, edges };
  }, [sessions, t, limit, hideImported]);

  const onNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => {
      if (preview || node.id === "start") {
        return;
      }
      void navigate({
        to: "/conversations",
        search: sessionChatSearch(node.id),
      });
    },
    [navigate, preview],
  );

  const heightClass = preview ? "h-[120px]" : "h-[280px]";

  return (
    <div
      className={`${heightClass} border border-outline-variant rounded bg-surface-container-low overflow-hidden session-flow ${
        preview ? "pointer-events-none opacity-90" : ""
      }`}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        proOptions={{ hideAttribution: true }}
        onNodeClick={onNodeClick}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={!preview}
        panOnDrag={!preview}
        zoomOnScroll={!preview}
        zoomOnPinch={!preview}
        zoomOnDoubleClick={!preview}
      >
        <Background className="session-flow-bg" color="var(--session-flow-grid)" gap={16} />
        {!preview && <Controls />}
      </ReactFlow>
    </div>
  );
}

function sessionBorderColor(s: SessionSummary): string {
  if (s.trusted_status === "blocked") {
    return "#ba1a1a";
  }
  if (s.trusted_status === "verified") {
    return "#16a34a";
  }
  return KIND_COLORS[s.kind] ?? "#ca8a04";
}

function nodeStyle(color: string): CSSProperties {
  return {
    background: "var(--session-flow-node-bg)",
    border: `1px solid ${color}`,
    color: "var(--session-flow-node-text)",
    fontSize: 12,
    padding: 8,
    borderRadius: 8,
    whiteSpace: "pre-wrap",
    maxWidth: 180,
    boxShadow: "var(--session-flow-node-shadow)",
    cursor: "pointer",
  };
}
