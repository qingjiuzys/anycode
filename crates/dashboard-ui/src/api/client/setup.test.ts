import { describe, expect, it } from "vitest";
import { setupClient } from "@/api/client/setup";

describe("setupClient", () => {
  it("defines setup status endpoint", () => {
    expect(setupClient.setupStatus).toBeTypeOf("function");
    expect(setupClient.setupComplete).toBeTypeOf("function");
    expect(setupClient.setupMemory).toBeTypeOf("function");
  });
});
