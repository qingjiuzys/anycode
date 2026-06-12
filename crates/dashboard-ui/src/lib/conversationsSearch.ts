export type ConversationSearch = {
  status?: string;
  trusted?: string;
  kind?: string;
  needs_approval?: boolean;
  budget_exceeded?: boolean;
  project?: string;
  session?: string;
  agent?: string;
  filter?: string;
};

/** Read active filter chip directly from the location search string (instant, no router merge lag). */
export function parseFilterFromSearchStr(searchStr: string): string {
  const params = new URLSearchParams(searchStr.startsWith("?") ? searchStr.slice(1) : searchStr);
  const f = params.get("filter")?.trim();
  if (f) return f;
  if (params.get("needs_approval") === "true" || params.get("needs_approval") === "1") {
    return "needs_approval";
  }
  if (params.get("trusted") === "blocked") return "blocked";
  if (params.get("budget_exceeded") === "true" || params.get("budget_exceeded") === "1") {
    return "budget";
  }
  if (params.get("status") === "running" && !params.get("kind") && !params.get("trusted")) {
    return "running";
  }
  const kind = params.get("kind");
  if (kind === "workflow" || kind === "cron") return kind;
  if (kind) return `kind:${kind}`;
  return "all";
}

/** Map filter chip id → API / list query fields. */
export function filterToQuerySearch(filter: string): ConversationSearch {
  if (filter === "all") return {};
  if (filter === "needs_approval") {
    return { filter, status: "running", needs_approval: true };
  }
  if (filter === "blocked") return { filter, trusted: "blocked" };
  if (filter === "budget") return { filter, budget_exceeded: true };
  if (filter === "running") return { filter, status: "running" };
  if (filter === "workflow" || filter === "cron") return { filter, kind: filter };
  if (filter.startsWith("kind:")) {
    const kind = filter.slice("kind:".length);
    return kind ? { filter, kind } : {};
  }
  return { filter };
}

/** Plain search object for navigate — replaces all params (never use a function updater). */
export function conversationSearchParams(search: ConversationSearch): ConversationSearch {
  const out: ConversationSearch = {};
  if (search.project) out.project = search.project;
  if (search.session) out.session = search.session;
  if (search.agent) out.agent = search.agent;
  if (search.filter) {
    out.filter = search.filter;
    return out;
  }
  if (search.status) out.status = search.status;
  if (search.trusted) out.trusted = search.trusted;
  if (search.kind) out.kind = search.kind;
  if (search.needs_approval) out.needs_approval = true;
  if (search.budget_exceeded) out.budget_exceeded = true;
  return out;
}

export function searchToSessionOpts(
  search: ConversationSearch,
  projectId?: string,
): {
  limit: number;
  status?: string;
  trustedStatus?: string;
  kind?: string;
  projectId?: string;
  budgetExceeded?: boolean;
} {
  return {
    limit: 100,
    status: search.status,
    trustedStatus: search.trusted,
    kind: search.kind,
    projectId: search.project ?? projectId,
    budgetExceeded: search.budget_exceeded,
  };
}

/** Full path + query for navigate/Link — replaces the entire search string (no param merge). */
export function buildConversationsHref(search: ConversationSearch = {}): string {
  const canon = conversationSearchParams(search);
  const params = new URLSearchParams();
  if (canon.filter) {
    params.set("filter", canon.filter);
  } else {
    if (canon.status) params.set("status", canon.status);
    if (canon.trusted) params.set("trusted", canon.trusted);
    if (canon.kind) params.set("kind", canon.kind);
    if (canon.needs_approval) params.set("needs_approval", "true");
    if (canon.budget_exceeded) params.set("budget_exceeded", "true");
  }
  if (canon.project) params.set("project", canon.project);
  if (canon.session) params.set("session", canon.session);
  if (canon.agent) params.set("agent", canon.agent);
  const q = params.toString();
  return q ? `/conversations?${q}` : "/conversations";
}

/** Parse project/session/agent + filter from a location search string. */
export function parseConversationSearch(searchStr: string): ConversationSearch {
  const raw = searchStr.startsWith("?") ? searchStr.slice(1) : searchStr;
  const params = new URLSearchParams(raw);
  const base: ConversationSearch = {
    project: params.get("project") ?? undefined,
    session: params.get("session") ?? undefined,
    agent: params.get("agent") ?? undefined,
  };
  const filter = params.get("filter")?.trim() || undefined;
  if (filter) return { ...base, filter };
  return { ...base, ...filterToQuerySearch(parseFilterFromSearchStr(searchStr)) };
}

/** If the URL mixes `filter` with legacy params, return the canonical href to redirect to. */
export function conversationsCanonicalHref(searchStr: string): string | null {
  const raw = searchStr.startsWith("?") ? searchStr.slice(1) : searchStr;
  const params = new URLSearchParams(raw);
  const legacyKeys = ["needs_approval", "trusted", "budget_exceeded", "status", "kind"] as const;
  const hasLegacy = legacyKeys.some((k) => params.has(k));
  const filter = params.get("filter")?.trim() || undefined;

  if (!hasLegacy && !filter) return null;

  const desired = conversationSearchParams(parseConversationSearch(searchStr));
  const href = buildConversationsHref(desired);
  const current = raw ? `/conversations?${raw}` : "/conversations";
  return href === current ? null : href;
}
