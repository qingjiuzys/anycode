/** Resolve control-center paths (including nested feature routes). */

export type ControlCenterView =
  | { view: "home" }
  | { view: "overview" }
  | { view: "projects" }
  | { view: "project"; projectId: string }
  | { view: "automations" }
  | { view: "assets"; search: Record<string, string> }
  | { view: "artifact"; artifactId: string }
  | { view: "reports"; search: Record<string, string> }
  | { view: "audit" }
  | { view: "account" }
  | { view: "agents" }
  | { view: "skill"; skillId: string }
  | { view: "settings"; search: Record<string, string> }
  | { view: "unknown" };

const TOP_LEVEL = new Set([
  "/",
  "/overview",
  "/projects",
  "/automations",
  "/assets",
  "/reports",
  "/audit",
  "/account",
  "/agents",
  "/settings",
]);

export function isControlCenterPath(pathname: string): boolean {
  if (TOP_LEVEL.has(pathname)) return true;
  if (/^\/projects\/[^/]+$/.test(pathname)) return true;
  if (/^\/assets\/[^/]+$/.test(pathname)) return true;
  if (/^\/agents\/[^/]+$/.test(pathname)) return true;
  return false;
}

export function shouldOpenControlCenterForLocation(pathname: string, _search = ""): boolean {
  if (pathname === "/conversations" || pathname.startsWith("/conversations/")) return false;
  if (pathname === "/login" || pathname === "/setup") return false;
  if (pathname.startsWith("/events/")) return false;
  if (pathname.startsWith("/sessions/")) return false;
  return isControlCenterPath(pathname);
}

export function controlCenterHref(pathname: string, search = ""): string {
  const q = search.startsWith("?") ? search : search ? `?${search}` : "";
  return `${pathname}${q}`;
}

export function parseControlCenterPath(path: string): ControlCenterView {
  const [pathname, rawQuery = ""] = path.split("?");
  const params = new URLSearchParams(rawQuery);
  const search: Record<string, string> = {};
  params.forEach((v, k) => {
    search[k] = v;
  });

  const project = pathname.match(/^\/projects\/([^/]+)$/);
  if (project) return { view: "project", projectId: decodeURIComponent(project[1]!) };

  const artifact = pathname.match(/^\/assets\/([^/]+)$/);
  if (artifact) return { view: "artifact", artifactId: decodeURIComponent(artifact[1]!) };

  const skill = pathname.match(/^\/agents\/([^/]+)$/);
  if (skill) return { view: "skill", skillId: decodeURIComponent(skill[1]!) };

  switch (pathname) {
    case "/":
      return { view: "home" };
    case "/overview":
      return { view: "overview" };
    case "/projects":
      return { view: "projects" };
    case "/automations":
      return { view: "automations" };
    case "/assets":
      return { view: "assets", search };
    case "/reports":
      return { view: "reports", search };
    case "/audit":
      return { view: "audit" };
    case "/account":
      return { view: "account" };
    case "/agents":
      return { view: "agents" };
    case "/settings":
      return { view: "settings", search };
    default:
      return { view: "unknown" };
  }
}

/** Build redirect target when a feature URL should open in control center. */
export function controlCenterRedirectTarget(pathname: string, searchStr = ""): {
  to: "/conversations";
  search: { cc: string };
} {
  return {
    to: "/conversations",
    search: { cc: controlCenterHref(pathname, searchStr) },
  };
}

export function buildControlCenterHref(
  to: string,
  params?: Record<string, string>,
  search?: Record<string, string | undefined>,
): string {
  let pathname = to;
  if (params) {
    for (const [key, value] of Object.entries(params)) {
      pathname = pathname.replace(`$${key}`, encodeURIComponent(value));
    }
  }
  const q = new URLSearchParams();
  if (search) {
    for (const [key, value] of Object.entries(search)) {
      if (value !== undefined && value !== "") q.set(key, value);
    }
  }
  const qs = q.toString();
  return qs ? `${pathname}?${qs}` : pathname;
}
