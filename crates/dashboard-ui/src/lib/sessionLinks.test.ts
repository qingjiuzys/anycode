import { describe, expect, it } from "vitest";
import { sessionChatSearch, sessionDetailSearch } from "@/lib/sessionLinks";

describe("sessionLinks", () => {
  it("builds conversations search for chat", () => {
    expect(sessionChatSearch("sess-1", "proj-1")).toEqual({
      session: "sess-1",
      project: "proj-1",
    });
  });

  it("builds debug tab search", () => {
    expect(sessionDetailSearch("audit")).toEqual({ tab: "audit" });
  });
});
