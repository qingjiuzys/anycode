/** Fixed reveal speed for streaming assistant text (chars per second). */
export const STREAM_REVEAL_CHARS_PER_SEC = 40;

/** @deprecated Use STREAM_REVEAL_CHARS_PER_SEC. Kept for tests importing legacy names. */
export const SMOOTH_BASE_CHARS_PER_SEC = STREAM_REVEAL_CHARS_PER_SEC;

/**
 * Advance displayed text toward target at a fixed rate.
 * Model/SSE may arrive faster; UI reveals steadily for readable output.
 */
export function smoothTextStep(
  displayed: string,
  target: string,
  deltaMs: number,
): string {
  if (target.length === 0) {
    return "";
  }
  if (!target.startsWith(displayed)) {
    displayed = "";
  }
  if (displayed.length >= target.length) {
    return target;
  }

  const chars = Math.max(
    1,
    Math.floor((STREAM_REVEAL_CHARS_PER_SEC * Math.max(deltaMs, 0)) / 1000),
  );
  return target.slice(0, Math.min(displayed.length + chars, target.length));
}
