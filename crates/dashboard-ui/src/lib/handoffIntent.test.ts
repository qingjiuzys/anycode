import { describe, expect, it } from "vitest";
import {
  buildHandoffColleaguesPath,
  parseHandoffIntent,
} from "@/lib/handoffIntent";

describe("handoffIntent", () => {
  it("parses project handoff intent", () => {
    expect(
      parseHandoffIntent({ handoff: "project", projectId: "p1" }),
    ).toEqual({ kind: "project", projectId: "p1" });
  });

  it("parses session handoff intent", () => {
    expect(
      parseHandoffIntent({
        handoff: "session",
        projectId: "p1",
        sessionId: "s1",
      }),
    ).toEqual({ kind: "session", projectId: "p1", sessionId: "s1" });
  });

  it("rejects incomplete intents", () => {
    expect(parseHandoffIntent({ handoff: "session", projectId: "p1" })).toBeNull();
    expect(parseHandoffIntent({ handoff: "project" })).toBeNull();
    expect(parseHandoffIntent(undefined)).toBeNull();
  });

  it("builds control-center colleagues path", () => {
    expect(
      buildHandoffColleaguesPath({
        kind: "session",
        projectId: "p1",
        sessionId: "s1",
      }),
    ).toBe("/colleagues?handoff=session&projectId=p1&sessionId=s1");
  });
});
