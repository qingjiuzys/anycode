import { describe, expect, it } from "vitest";
import {
  agentDisplayLabel,
  agentLabelKey,
  isPrimaryAgentId,
  normalizeAgentId,
} from "./agentCatalog";

describe("agentCatalog", () => {
  const t = (key: string) => {
    const labels: Record<string, string> = {
      "agents.builtin.generalPurpose": "General purpose",
      "agents.builtin.workspaceAssistant": "Workspace assistant",
      "agents.builtin.channelOps": "Channel ops",
    };
    return labels[key] ?? key;
  };

  it("maps known agent ids to label keys", () => {
    expect(agentLabelKey("workspace-assistant")).toBe("workspaceAssistant");
    expect(agentLabelKey("channel-ops")).toBe("channelOps");
  });

  it("returns localized labels with id fallback", () => {
    expect(agentDisplayLabel("general-purpose", t)).toBe("General purpose");
    expect(agentDisplayLabel("custom-agent", t)).toBe("custom-agent");
  });

  it("normalizes deprecated aliases", () => {
    expect(normalizeAgentId("builder")).toBe("general-purpose");
    expect(normalizeAgentId("goal-runner")).toBe("goal");
    expect(isPrimaryAgentId("builder")).toBe(true);
  });
});
