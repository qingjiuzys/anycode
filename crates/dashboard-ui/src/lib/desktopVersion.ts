/** Fetch portal latest.json and compare with local app version. */

export type DesktopLatestManifest = {
  version: string;
  arch?: string;
  filename?: string;
  url?: string;
  latest_url?: string;
  sha256?: string;
};

function normalizeVersion(v: string): string {
  return v.trim().replace(/^v/i, "");
}

/** Semver-ish compare: returns 1 if a>b, -1 if a<b, 0 if equal/unknown. */
export function compareSemver(a: string, b: string): number {
  const pa = normalizeVersion(a)
    .split(/[.+-]/)
    .map((p) => Number.parseInt(p, 10));
  const pb = normalizeVersion(b)
    .split(/[.+-]/)
    .map((p) => Number.parseInt(p, 10));
  const n = Math.max(pa.length, pb.length);
  for (let i = 0; i < n; i++) {
    const x = Number.isFinite(pa[i]) ? pa[i]! : 0;
    const y = Number.isFinite(pb[i]) ? pb[i]! : 0;
    if (x > y) return 1;
    if (x < y) return -1;
  }
  return 0;
}

export async function fetchDesktopLatest(
  portalOrigin: string,
): Promise<DesktopLatestManifest> {
  const base = portalOrigin.replace(/\/$/, "");
  const res = await fetch(`${base}/downloads/latest.json`, {
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error(`latest.json HTTP ${res.status}`);
  }
  return (await res.json()) as DesktopLatestManifest;
}
