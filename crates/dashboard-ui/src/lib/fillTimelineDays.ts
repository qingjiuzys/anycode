import type { GlobalTimelineMetrics, TimelineMetricPoint, TokenTimelinePoint } from "@/api/types";

/** UTC calendar keys for a window ending today (oldest → newest). */
export function calendarDayKeys(days: number): string[] {
  const n = Math.max(1, Math.floor(days));
  const today = new Date();
  const y = today.getUTCFullYear();
  const m = today.getUTCMonth();
  const d = today.getUTCDate();
  const keys: string[] = [];
  for (let offset = n - 1; offset >= 0; offset--) {
    keys.push(new Date(Date.UTC(y, m, d - offset)).toISOString().slice(0, 10));
  }
  return keys;
}

/** Ensure token timeline has exactly `days` points, zero-filled for missing days. */
export function fillTokenTimelineDays(
  points: TokenTimelinePoint[],
  days: number,
): TokenTimelinePoint[] {
  const byDate = new Map(points.map((p) => [p.date.slice(0, 10), p]));
  return calendarDayKeys(days).map(
    (date) =>
      byDate.get(date) ?? {
        date,
        llm_calls: 0,
        input_tokens: 0,
        output_tokens: 0,
        total_tokens: 0,
        estimated_cost_cny: 0,
      },
  );
}

/** Ensure home throughput timeline has exactly `days` points, zero-filled. */
export function fillHomeTimelineDays(
  timeline: GlobalTimelineMetrics | undefined,
  days = 7,
): GlobalTimelineMetrics | undefined {
  if (!timeline) return undefined;
  const byDate = new Map(timeline.points.map((p) => [p.date.slice(0, 10), p]));
  const points: TimelineMetricPoint[] = calendarDayKeys(days).map(
    (date) =>
      byDate.get(date) ?? {
        date,
        sessions_count: 0,
        events_count: 0,
        gates_failed: 0,
      },
  );
  return { ...timeline, days, points };
}
