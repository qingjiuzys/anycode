import { describe, expect, it } from "vitest";
import {
  defaultCollapsedProjectIds,
  groupSessionsByProject,
  projectGroupActivityAt,
} from "@/lib/groupSessionsByProject";
import type { SessionWithProject } from "@/api/types";

function session(overrides: Partial<SessionWithProject>): SessionWithProject {
  return {
    id: "s1",
    project_id: "p1",
    project_name: "Alpha",
    kind: "repl",
    task_id: null,
    title: "Session",
    status: "completed",
    trusted_status: "ok",
    agent_type: "general-purpose",
    model: "glm-5",
    started_at: "2026-01-01T10:00:00Z",
    ended_at: null,
    ...overrides,
  };
}

describe("groupSessionsByProject", () => {
  it("sorts sessions within a group by recency", () => {
    const groups = groupSessionsByProject([{ id: "p1", name: "Alpha" }], [
      session({ id: "s-old", project_id: "p1", started_at: "2026-01-01T10:00:00Z" }),
      session({ id: "s-new", project_id: "p1", started_at: "2026-01-02T10:00:00Z" }),
    ]);
    expect(groups[0]!.sessions.map((s) => s.id)).toEqual(["s-new", "s-old"]);
  });

  it("floats running sessions to the top of a project group", () => {
    const groups = groupSessionsByProject([{ id: "p1", name: "Alpha" }], [
      session({ id: "s-older", project_id: "p1", started_at: "2026-01-01T10:00:00Z" }),
      session({
        id: "s-running",
        project_id: "p1",
        status: "running",
        started_at: "2026-01-01T09:00:00Z",
      }),
      session({ id: "s-newest", project_id: "p1", started_at: "2026-01-03T10:00:00Z" }),
    ]);
    expect(groups[0]!.sessions.map((s) => s.id)).toEqual([
      "s-running",
      "s-newest",
      "s-older",
    ]);
  });

  it("orders project groups by latest session activity", () => {
    const groups = groupSessionsByProject(
      [
        { id: "p1", name: "Alpha" },
        { id: "p2", name: "Beta" },
      ],
      [
        session({ id: "s1", project_id: "p1", started_at: "2026-01-01T10:00:00Z" }),
        session({ id: "s2", project_id: "p2", started_at: "2026-01-03T10:00:00Z" }),
      ],
    );
    expect(groups.map((g) => g.id)).toEqual(["p2", "p1"]);
  });

  it("bubbles running projects using project updated_at over stale started_at", () => {
    const groups = groupSessionsByProject(
      [
        { id: "p1", name: "Alpha", updated_at: "2026-01-01T12:00:00Z" },
        { id: "p2", name: "Beta", updated_at: "2026-01-05T08:00:00Z" },
      ],
      [
        session({
          id: "s-old-run",
          project_id: "p2",
          project_name: "Beta",
          status: "running",
          started_at: "2026-01-01T10:00:00Z",
        }),
        session({ id: "s1", project_id: "p1", started_at: "2026-01-04T10:00:00Z" }),
      ],
    );
    expect(groups.map((g) => g.id)).toEqual(["p2", "p1"]);
    expect(
      projectGroupActivityAt(groups[0]!, "2026-01-05T08:00:00Z"),
    ).toBe("2026-01-05T08:00:00Z");
  });

  it("includes empty projects after active ones, ordered by updated_at", () => {
    const groups = groupSessionsByProject(
      [
        { id: "p1", name: "Alpha", updated_at: "2026-01-02T10:00:00Z" },
        { id: "p2", name: "Beta", updated_at: "2026-01-05T10:00:00Z" },
        { id: "p3", name: "Gamma", updated_at: "2026-01-01T10:00:00Z" },
      ],
      [session({ id: "s1", project_id: "p1", started_at: "2026-01-03T10:00:00Z" })],
    );
    expect(groups.map((g) => g.id)).toEqual(["p2", "p1", "p3"]);
  });

  it("collapses all but first two projects by default", () => {
    const groups = groupSessionsByProject(
      [
        { id: "p1", name: "A" },
        { id: "p2", name: "B" },
        { id: "p3", name: "C" },
      ],
      [],
    );
    expect(defaultCollapsedProjectIds(groups)).toEqual(new Set(["p3"]));
  });

  it("keeps pinned projects ahead of activity order", () => {
    const groups = groupSessionsByProject(
      [
        { id: "p1", name: "Alpha" },
        { id: "p2", name: "Beta" },
      ],
      [
        session({ id: "s1", project_id: "p1", started_at: "2026-01-01T10:00:00Z" }),
        session({ id: "s2", project_id: "p2", started_at: "2026-01-03T10:00:00Z" }),
      ],
      new Set(["p1"]),
    );
    expect(groups.map((g) => g.id)).toEqual(["p1", "p2"]);
  });

  it("does not resurrect archived projects from leftover sessions", () => {
    const groups = groupSessionsByProject(
      [{ id: "p1", name: "Alpha" }],
      [
        session({ id: "s1", project_id: "p1", started_at: "2026-01-03T10:00:00Z" }),
        session({
          id: "s-gone",
          project_id: "p-archived",
          project_name: "Gone",
          started_at: "2026-01-04T10:00:00Z",
        }),
      ],
    );
    expect(groups.map((g) => g.id)).toEqual(["p1"]);
  });
});
