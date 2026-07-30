import { describe, expect, it } from "vitest";
import {
  buildColleaguesGraph,
  colleagueInitial,
  demoColleagues,
} from "./colleaguesGraph";

describe("colleaguesGraph", () => {
  it("builds a self node plus circular peers with names", () => {
    const peers = demoColleagues();
    const { nodes, edges } = buildColleaguesGraph("我", peers);
    expect(nodes).toHaveLength(1 + peers.length);
    expect(nodes[0]?.data.kind).toBe("self");
    expect(nodes[0]?.data.initial).toBe("我");
    expect(nodes.slice(1).every((n) => n.data.kind === "peer")).toBe(true);
    expect(nodes.slice(1).map((n) => n.data.name)).toEqual(
      peers.map((p) => p.name),
    );
    expect(edges).toHaveLength(peers.length);
  });

  it("colleagueInitial uses first character", () => {
    expect(colleagueInitial("林晓")).toBe("林");
    expect(colleagueInitial("alice")).toBe("A");
    expect(colleagueInitial("  ")).toBe("?");
  });
});
