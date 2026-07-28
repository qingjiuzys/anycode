import type { Edge, Node } from "reactflow";
import type { LanPeer } from "@/api/client/lan";

export type ColleagueNodeKind = "self" | "peer";

export type ColleagueNodeData = {
  kind: ColleagueNodeKind;
  name: string;
  subtitle?: string;
  peerId?: string;
  online?: boolean;
};

const SELF_NODE_ID = "self";
const RADIUS = 240;
const CENTER = { x: 0, y: 0 };

function peerPosition(index: number, total: number): { x: number; y: number } {
  if (total <= 0) return { x: RADIUS, y: 0 };
  const angle = (Math.PI * 2 * index) / total - Math.PI / 2;
  return {
    x: CENTER.x + RADIUS * Math.cos(angle),
    y: CENTER.y + RADIUS * Math.sin(angle),
  };
}

export function buildColleaguesGraph(
  selfName: string,
  peers: LanPeer[],
): { nodes: Node<ColleagueNodeData>[]; edges: Edge[] } {
  const nodes: Node<ColleagueNodeData>[] = [
    {
      id: SELF_NODE_ID,
      type: "colleague",
      position: CENTER,
      data: {
        kind: "self",
        name: selfName,
        subtitle: "LAN",
        online: true,
      },
      draggable: false,
    },
  ];

  const edges: Edge[] = peers.map((peer) => {
    const pos = peerPosition(nodes.length - 1, peers.length);
    nodes.push({
      id: peer.instance_id,
      type: "colleague",
      position: pos,
      data: {
        kind: "peer",
        name: peer.device_name,
        subtitle: `${peer.host}:${peer.lan_port} · v${peer.version}`,
        peerId: peer.instance_id,
        online: true,
      },
      draggable: false,
    });
    return {
      id: `edge-${SELF_NODE_ID}-${peer.instance_id}`,
      source: SELF_NODE_ID,
      target: peer.instance_id,
      type: "smoothstep",
      animated: true,
      style: { stroke: "var(--colleague-edge, #94a3b8)", strokeWidth: 1.5, strokeDasharray: "6 4" },
    };
  });

  return { nodes, edges };
}

export function colleagueGraphFitPadding(peerCount: number): number {
  return peerCount === 0 ? 0.35 : 0.25;
}
