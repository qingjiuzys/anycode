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
import {
  buildColleaguesGraph,
  colleagueGraphFitPadding,
  type ColleagueNodeData,
  type GraphPeer,
} from "@/lib/colleaguesGraph";
import { useT } from "@/i18n/context";

type Props = {
  selfName: string;
  peers: GraphPeer[];
  selectedPeerId?: string | null;
  /** Left-click or right-click a peer — open handoff menu at pointer. */
  onPeerInteract?: (peerId: string, event: React.MouseEvent) => void;
};

function ColleagueNodeView({ data, selected }: NodeProps<ColleagueNodeData>) {
  const isSelf = data.kind === "self";
  const accent = isSelf ? "var(--primary)" : "var(--secondary)";

  return (
    <div
      className={`dw-colleague-node${isSelf ? " dw-colleague-node--self" : ""}${
        selected ? " dw-colleague-node--selected" : ""
      }${data.demo ? " dw-colleague-node--demo" : ""}`}
      style={{ "--colleague-accent": accent } as CSSProperties}
      data-kind={data.kind}
      title={data.demo ? undefined : data.name}
    >
      <Handle type="target" position={Position.Top} className="!opacity-0 !h-1 !w-1 !min-w-0 !min-h-0" />
      <div className="dw-colleague-node__circle" aria-hidden>
        <span className="dw-colleague-node__initial">{data.initial}</span>
      </div>
      <div className="dw-colleague-node__name">{data.name}</div>
      <Handle type="source" position={Position.Bottom} className="!opacity-0 !h-1 !w-1 !min-w-0 !min-h-0" />
    </div>
  );
}

const nodeTypes = { colleague: ColleagueNodeView };

export const ColleaguesGraph = memo(function ColleaguesGraph({
  selfName,
  peers,
  selectedPeerId,
  onPeerInteract,
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

  const interact = (event: React.MouseEvent, node: Node<ColleagueNodeData>) => {
    if (node.data.kind === "self") return;
    const peerId = node.data.peerId ?? node.id;
    onPeerInteract?.(peerId, event);
  };

  const onNodeClick: NodeMouseHandler = (event, node) => {
    interact(event, node as Node<ColleagueNodeData>);
  };

  const onNodeContextMenu: NodeMouseHandler = (event, node) => {
    event.preventDefault();
    interact(event, node as Node<ColleagueNodeData>);
  };

  return (
    <div className="dw-colleagues-graph">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodeClick={onNodeClick}
        onNodeContextMenu={onNodeContextMenu}
        fitView
        fitViewOptions={{ padding: colleagueGraphFitPadding(peers.length) }}
        minZoom={0.4}
        maxZoom={1.4}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        panOnDrag
        proOptions={{ hideAttribution: true }}
      >
        <Background gap={22} size={1} color="var(--outline-variant)" />
        <Controls showInteractive={false} />
      </ReactFlow>
      <div className="dw-colleagues-graph__legend">
        <span>{t("colleagues.graphLegend")}</span>
        <span className="text-on-surface-variant">
          {peers.every((p) => p.demo)
            ? t("colleagues.demoHint")
            : peers.length === 0
              ? t("colleagues.emptyShort")
              : t("colleagues.peerCount").replace("{count}", String(peers.length))}
        </span>
        <span className="text-on-surface-variant">{t("colleagues.clickOrRightHint")}</span>
      </div>
    </div>
  );
});
