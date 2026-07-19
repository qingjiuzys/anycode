import { memo, useCallback, useEffect, useMemo, useState, type CSSProperties } from "react";
import ReactFlow, {
  Background,
  Controls,
  Handle,
  Position,
  type Edge,
  type Node,
  type NodeMouseHandler,
  type NodeProps,
} from "reactflow";
import "reactflow/dist/style.css";
import type { TranscriptBlock } from "@/api/types";
import {
  buildExecutionTraceGraph,
  findRunningToolNodeId,
  type TraceGraphNodeData,
  type TraceNodeKind,
  type TraceNodeStatus,
} from "@/lib/executionTraceGraph";
import { useT } from "@/i18n/context";

type Props = {
  blocks: TranscriptBlock[];
  isRunning?: boolean;
  selectedToolId?: string | null;
  onSelectTool?: (tool: TranscriptBlock | null) => void;
};

const KIND_ACCENT: Record<TraceNodeKind, string> = {
  user: "#64748b",
  assistant: "#2563eb",
  tool: "#7c3aed",
  decision: "#0891b2",
  branch: "#64748b",
};

const STATUS_ACCENT: Partial<Record<TraceNodeStatus, string>> = {
  ok: "#16a34a",
  failed: "#ba1a1a",
  running: "#ca8a04",
  chosen: "#2563eb",
  skipped: "#94a3b8",
};

function resolveAccent(data: TraceGraphNodeData): string {
  if (data.status && STATUS_ACCENT[data.status]) {
    return STATUS_ACCENT[data.status]!;
  }
  if (data.failed) return STATUS_ACCENT.failed!;
  if (data.running) return STATUS_ACCENT.running!;
  return KIND_ACCENT[data.kind];
}

function TraceNodeView({ data, selected }: NodeProps<TraceGraphNodeData>) {
  const accent = resolveAccent(data);
  const pulse = data.running || data.live || data.status === "running";

  return (
    <div
      className={`dw-trace-node ${pulse ? "dw-trace-node--pulse" : ""}`}
      style={
        {
          "--trace-accent": accent,
          outline: selected ? `2px solid ${accent}` : undefined,
        } as CSSProperties
      }
      data-kind={data.kind}
      data-status={data.status ?? "neutral"}
      data-failed={data.failed ? "true" : undefined}
      data-chosen={data.chosen ? "true" : undefined}
    >
      <Handle type="target" position={Position.Top} className="!opacity-0 !h-1 !w-1 !min-w-0 !min-h-0" />
      <div className="dw-trace-node__kind">{data.label}</div>
      {data.subtitle ? (
        <div className="dw-trace-node__sub font-code">{data.subtitle}</div>
      ) : null}
      <Handle type="source" position={Position.Bottom} className="!opacity-0 !h-1 !w-1 !min-w-0 !min-h-0" />
    </div>
  );
}

const nodeTypes = { trace: TraceNodeView };

export const ExecutionTraceGraph = memo(function ExecutionTraceGraph({
  blocks,
  isRunning = false,
  selectedToolId,
  onSelectTool,
}: Props) {
  const t = useT();
  const model = useMemo(() => buildExecutionTraceGraph(blocks), [blocks]);
  const runningId = useMemo(
    () => (isRunning ? findRunningToolNodeId(model.nodes) : null),
    [isRunning, model.nodes],
  );

  const [nodes, setNodes] = useState<Node<TraceGraphNodeData>[]>(model.nodes);
  const [edges, setEdges] = useState<Edge[]>(model.edges);

  useEffect(() => {
    setNodes(
      model.nodes.map((n) => ({
        ...n,
        selected: Boolean(
          selectedToolId &&
            (n.data.block?.id === selectedToolId || n.id === selectedToolId),
        ),
        data: {
          ...n.data,
          running: n.data.running || n.id === runningId,
          label:
            n.data.kind === "user"
              ? t("conversations.traceNodeUser")
              : n.data.kind === "assistant"
                ? n.data.failed
                  ? t("common.error")
                  : t("conversations.traceNodeAssistant")
                : n.data.kind === "decision"
                  ? n.data.label || t("conversations.traceNodeDecision")
                  : n.data.kind === "branch" && n.data.branchKey === "allow"
                    ? t("conversations.traceBranchAllow")
                    : n.data.kind === "branch" && n.data.branchKey === "deny"
                      ? t("conversations.traceBranchDeny")
                      : n.data.label,
        },
      })),
    );
    setEdges(model.edges);
  }, [model, runningId, selectedToolId, t]);

  const onNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => {
      const data = node.data as TraceGraphNodeData;
      if (!data.block) return;
      if (data.kind === "tool" || data.kind === "decision" || data.kind === "branch") {
        onSelectTool?.(data.block);
      }
    },
    [onSelectTool],
  );

  if (model.nodes.length === 0) {
    return (
      <p className="text-xs text-secondary m-0 px-3 py-4">
        {t("conversations.inspectorTimelineEmpty")}
      </p>
    );
  }

  const maxY = model.nodes.reduce((m, n) => Math.max(m, n.position.y), 0);
  const heightPx = Math.min(480, Math.max(200, maxY + 120));

  return (
    <div
      className="dw-execution-trace-graph border-b border-outline-variant/60 bg-surface-container-low/30"
      style={{ height: heightPx }}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.18, maxZoom: 1.1 }}
        proOptions={{ hideAttribution: true }}
        onNodeClick={onNodeClick}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        panOnDrag
        zoomOnScroll
        zoomOnPinch
        minZoom={0.4}
        maxZoom={1.4}
      >
        <Background gap={14} size={1} color="var(--color-outline-variant, #e2e8f0)" />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
});
