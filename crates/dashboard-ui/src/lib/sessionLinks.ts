import { conversationSearchParams, type ConversationSearch } from "@/lib/conversationsSearch";

export type SessionDetailTab = "debug" | "audit";

/** Search params for opening a session in the conversations workspace. */
export function sessionChatSearch(
  sessionId: string,
  projectId?: string,
): ConversationSearch {
  const search: ConversationSearch = { session: sessionId };
  if (projectId) search.project = projectId;
  return conversationSearchParams(search);
}

/** Search params for session debug/audit pages (not redirected to conversations). */
export function sessionDetailSearch(tab: SessionDetailTab = "debug"): { tab: SessionDetailTab } {
  return { tab };
}
