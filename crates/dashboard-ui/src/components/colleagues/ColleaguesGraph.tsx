import { memo, useEffect, useMemo, useState, type CSSProperties } from "react";
import ReactFlow, {
  Background,
  Controls,
  Handle,
  Position,
  type Node,
  type NodeMouseHandler,
  type NodeProps,
} from "reactflow";
import "reactflow/dist/style.css";
import type { LanPeer } from "@/api/client/lan";
import {
  buildColleaguesGraph,
  colleagueGraphFitPadding,
  type ColleagueNodeData,
} from "@/lib/colleaguesGraph";
import { useT } from "@/i18n/context";

type Props = {
  selfName: string;
  peers: LanPeer[];
  selectedPeerId?: string | null;
  onSelectPeer?: (peerId: string | null) => void;
};

function ColleagueNodeView({ data, selected }: NodeProps<ColleagueNodeData>) {
  const isSelf = data.kind === "self";
  const accent = isSelf ? "var(--primary)" : "var(--secondary)";

  return (
    <div
      className={`dw-colleague-node${isSelf ? " dw-colleague-node--self" : ""}${
        selected ? " dw-colleague-node--selected" : ""
      }`}
      style={{ "--colleague-accent": accent } as CSSProperties}
      data-kind={data.kind}
    >
      <Handle type="target" position={Position.Top} className="!opacity-0 !h-1 !w-1 !min-w-0 !min-h-0" />
      <div className="dw-colleague-node__avatar" aria-hidden>
        {isSelf ? "◎" : "◉"}
      </div>
      <div className="dw-colleague-node__name">{data.name}</div>
      {data.subtitle ? <div className="dw-colleague-node__sub">{data.subtitle}</div> : null}
      <Handle type="source" position={Position.Bottom} className="!opacity-0 !h-1 !w-1 !min-w-0 !min-h-0" />
    </div>
  );
}

const nodeTypes = { colleague: ColleagueNodeView };

export const ColleaguesGraph = memo(function ColleaguesGraph({
  selfName,
  peers,
  selectedPeerId,
  onSelectPeer,
}: Props) {
  const t = useT();
  const model = useMemo(() => buildColleaguesGraph(selfName, peers), [selfName, peers]);
  const [nodes, setNodes] = useState<Node<ColleagueNodeData>[]>(model.nodes);
  const [edges, setEdges] = useState(model.edges);

  useEffect(() => {
    setNodes(
      model.nodes.map((n) => ({
        ...n,
        selected: n.id === selectedPeerId || (selectedPeerId == null && n.data.kind === "self"),
      })),
    );
    setEdges(model.edges);
  }, [model, selectedPeerId]);

  const onNodeClick: NodeMouseHandler = (_event, node) => {
    if (node.data.kind === "self") {
      onSelectPeer?.(null);
      return;
    }
    onSelectPeer?.(node.id);
  };

  return (
    <div className="dw-colleagues-graph">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodeClick={onNodeClick}
        fitView
        fitViewOptions={{ padding: colleagueGraphFitPadding(peers.length) }}
        minZoom={0.4}
        maxZoom={1.4}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={20} size={1} color="var(--outline-variant)" />
        <Controls showInteractive={false} />
      </ReactFlow>
      <div className="dw-colleagues-graph__legend">
        <span>{t("colleagues.graphLegend")}</span>
        <span className="text-on-surface-variant">
          {peers.length === 0
            ? t("colleagues.emptyShort")
            : t("colleagues.peerCount").replace("{count}", String(peers.length))}
        </span>
      </div>
    </div>
  );
});
