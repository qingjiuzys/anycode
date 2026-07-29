import { describe, expect, it } from "vitest";
import { goalSlashCommand, isGoalSlashToken } from "./goalMode";

describe("goalMode", () => {
  it("locale slash command names", () => {
    expect(goalSlashCommand("zh")).toBe("目标");
    expect(goalSlashCommand("en")).toBe("goal");
  });

  it("recognizes slash tokens", () => {
    expect(isGoalSlashToken("目标")).toBe(true);
    expect(isGoalSlashToken("goal")).toBe(true);
    expect(isGoalSlashToken("goal-mode")).toBe(true);
    expect(isGoalSlashToken("help")).toBe(false);
  });
});
