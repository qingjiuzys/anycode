import { describe, expect, it } from "vitest";
import {
  buildControlCenterHref,
  controlCenterRedirectTarget,
  isControlCenterPath,
  parseControlCenterPath,
  shouldOpenControlCenterForLocation,
} from "@/lib/controlCenterPaths";

describe("controlCenterPaths", () => {
  it("detects top-level and nested paths", () => {
    expect(isControlCenterPath("/settings")).toBe(true);
    expect(isControlCenterPath("/projects/p1")).toBe(true);
    expect(isControlCenterPath("/assets/a1")).toBe(true);
    expect(isControlCenterPath("/agents/s1")).toBe(true);
    expect(isControlCenterPath("/conversations")).toBe(false);
    expect(isControlCenterPath("/sessions/x")).toBe(false);
  });

  it("parses nested detail views", () => {
    expect(parseControlCenterPath("/projects/p1")).toEqual({
      view: "project",
      projectId: "p1",
    });
    expect(parseControlCenterPath("/assets/a1")).toEqual({
      view: "artifact",
      artifactId: "a1",
    });
    expect(parseControlCenterPath("/agents/s1")).toEqual({
      view: "skill",
      skillId: "s1",
    });
    expect(parseControlCenterPath("/reports?artifact_id=r1")).toEqual({
      view: "reports",
      search: { artifact_id: "r1" },
    });
  });

  it("builds hrefs with params and search", () => {
    expect(
      buildControlCenterHref("/projects/$projectId", { projectId: "p1" }),
    ).toBe("/projects/p1");
    expect(
      buildControlCenterHref("/reports", undefined, { artifact_id: "r1" }),
    ).toBe("/reports?artifact_id=r1");
  });

  it("builds redirect target for deep links", () => {
    expect(controlCenterRedirectTarget("/settings", "?section=prefs")).toEqual({
      to: "/conversations",
      search: { cc: "/settings?section=prefs" },
    });
  });

  it("decides deep-link sync targets", () => {
    expect(shouldOpenControlCenterForLocation("/settings", "")).toBe(true);
    expect(shouldOpenControlCenterForLocation("/conversations", "")).toBe(false);
    expect(shouldOpenControlCenterForLocation("/sessions/s1", "?tab=debug")).toBe(
      false,
    );
    expect(shouldOpenControlCenterForLocation("/events/e1", "")).toBe(false);
  });
});
