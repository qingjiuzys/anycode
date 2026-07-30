import type { Edge, Node } from "reactflow";

export type ColleagueNodeKind = "self" | "peer";

export type GraphPeer = {
  id: string;
  name: string;
  subtitle?: string;
  /** Preview-only node — not a real cloud peer. */
  demo?: boolean;
};

export type ColleagueNodeData = {
  kind: ColleagueNodeKind;
  name: string;
  subtitle?: string;
  peerId?: string;
  online?: boolean;
  demo?: boolean;
  initial: string;
};

const SELF_NODE_ID = "self";
const RADIUS = 220;
const CENTER = { x: 0, y: 0 };

function peerPosition(index: number, total: number): { x: number; y: number } {
  if (total <= 0) return { x: RADIUS, y: 0 };
  const angle = (Math.PI * 2 * index) / total - Math.PI / 2;
  return {
    x: CENTER.x + RADIUS * Math.cos(angle),
    y: CENTER.y + RADIUS * Math.sin(angle),
  };
}

export function colleagueInitial(name: string): string {
  const trimmed = name.trim();
  if (!trimmed) return "?";
  return trimmed.slice(0, 1).toUpperCase();
}

/** Mock teammates for empty-state visual preview. */
export function demoColleagues(): GraphPeer[] {
  return [
    { id: "demo_lin", name: "林晓", subtitle: "预览", demo: true },
    { id: "demo_chen", name: "陈默", subtitle: "预览", demo: true },
    { id: "demo_zhou", name: "周予", subtitle: "预览", demo: true },
    { id: "demo_wang", name: "王可", subtitle: "预览", demo: true },
  ];
}

export function buildColleaguesGraph(
  selfName: string,
  peers: GraphPeer[],
): { nodes: Node<ColleagueNodeData>[]; edges: Edge[] } {
  const nodes: Node<ColleagueNodeData>[] = [
    {
      id: SELF_NODE_ID,
      type: "colleague",
      position: CENTER,
      data: {
        kind: "self",
        name: selfName,
        subtitle: undefined,
        online: true,
        initial: colleagueInitial(selfName),
      },
      draggable: false,
    },
  ];

  const edges: Edge[] = peers.map((peer, index) => {
    const pos = peerPosition(index, peers.length);
    nodes.push({
      id: peer.id,
      type: "colleague",
      position: pos,
      data: {
        kind: "peer",
        name: peer.name,
        subtitle: peer.subtitle,
        peerId: peer.id,
        online: true,
        demo: peer.demo,
        initial: colleagueInitial(peer.name),
      },
      draggable: false,
    });
    return {
      id: `edge-${SELF_NODE_ID}-${peer.id}`,
      source: SELF_NODE_ID,
      target: peer.id,
      type: "straight",
      animated: !peer.demo,
      style: {
        stroke: peer.demo
          ? "color-mix(in srgb, var(--outline-variant) 80%, transparent)"
          : "var(--colleague-edge, #94a3b8)",
        strokeWidth: 1.25,
        strokeDasharray: peer.demo ? "4 6" : undefined,
      },
    };
  });

  return { nodes, edges };
}

export function colleagueGraphFitPadding(peerCount: number): number {
  return peerCount === 0 ? 0.45 : 0.3;
}

export { SELF_NODE_ID };
