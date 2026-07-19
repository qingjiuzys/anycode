import { describe, expect, it } from "vitest";
import { formatDuration, formatRelativeTime, parseUtcTimestamp } from "@/utils/formatTime";

describe("parseUtcTimestamp", () => {
  const utcMs = Date.parse("2026-07-11T19:00:00.000Z");

  it("treats SQLite datetime strings as UTC", () => {
    expect(parseUtcTimestamp("2026-07-11 19:00:00")).toBe(utcMs);
  });

  it("treats naive ISO strings as UTC", () => {
    expect(parseUtcTimestamp("2026-07-11T19:00:00")).toBe(utcMs);
  });

  it("keeps explicit UTC suffixes", () => {
    expect(parseUtcTimestamp("2026-07-11T19:00:00Z")).toBe(utcMs);
    expect(parseUtcTimestamp("2026-07-11T19:00:00+00:00")).toBe(utcMs);
  });
});

describe("formatRelativeTime", () => {
  it("does not apply an 8-hour offset for naive UTC timestamps", () => {
    const now = Date.parse("2026-07-11T19:00:30.000Z");
    expect(formatRelativeTime("2026-07-11 19:00:00", now)).toBe("30s");
    expect(formatRelativeTime("2026-07-11T19:00:00", now)).toBe("30s");
  });
});

describe("formatDuration", () => {
  it("computes duration from naive UTC timestamps", () => {
    expect(formatDuration("2026-07-11 19:00:00", "2026-07-11 19:02:30")).toBe("2m 30s");
  });
});
