/** Parse API timestamps as UTC when no timezone suffix is present (SQLite `datetime('now')`). */
export function parseUtcTimestamp(iso: string): number {
  const trimmed = iso.trim();
  if (!trimmed) return Number.NaN;
  if (/[zZ]$|[+-]\d{2}:\d{2}$/.test(trimmed)) {
    return Date.parse(trimmed);
  }
  const normalized = trimmed.includes("T") ? trimmed : trimmed.replace(" ", "T");
  return Date.parse(`${normalized}Z`);
}

/** Relative time label for ISO timestamps (en/zh via caller). */
export function formatRelativeTime(iso: string, now = Date.now()): string {
  const ts = parseUtcTimestamp(iso);
  if (Number.isNaN(ts)) return iso;
  const diffSec = Math.round((now - ts) / 1000);
  if (diffSec < 60) return `${Math.max(0, diffSec)}s`;
  const diffMin = Math.round(diffSec / 60);
  if (diffMin < 60) return `${diffMin}m`;
  const diffHr = Math.round(diffMin / 60);
  if (diffHr < 48) return `${diffHr}h`;
  const diffDay = Math.round(diffHr / 24);
  return `${diffDay}d`;
}

export function formatDuration(startIso: string, endIso?: string | null): string {
  const start = parseUtcTimestamp(startIso);
  const end = endIso ? parseUtcTimestamp(endIso) : Date.now();
  if (Number.isNaN(start) || Number.isNaN(end)) return "—";
  const sec = Math.max(0, Math.round((end - start) / 1000));
  if (sec < 60) return `${sec}s`;
  const min = Math.floor(sec / 60);
  const rem = sec % 60;
  if (min < 60) return rem > 0 ? `${min}m ${rem}s` : `${min}m`;
  const hr = Math.floor(min / 60);
  return `${hr}h ${min % 60}m`;
}
