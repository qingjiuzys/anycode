import type { QueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";

export const SESSION_QUERY_GC_MS = 30 * 60_000;
export const TRANSCRIPT_STALE_RUNNING_MS = 3_000;

export function transcriptStaleTime(isRunning: boolean): number {
  return isRunning ? TRANSCRIPT_STALE_RUNNING_MS : Number.POSITIVE_INFINITY;
}

export function transcriptQueryOptions(sessionId: string, isRunning = false) {
  return {
    queryKey: ["session-transcript", sessionId] as const,
    queryFn: () => api.sessionTranscript(sessionId),
    staleTime: transcriptStaleTime(isRunning),
    gcTime: SESSION_QUERY_GC_MS,
  };
}

export function sessionArtifactsQueryOptions(sessionId: string, isRunning = false) {
  return {
    queryKey: ["session-artifacts", sessionId] as const,
    queryFn: () => api.sessionArtifacts(sessionId),
    staleTime: isRunning ? TRANSCRIPT_STALE_RUNNING_MS : Number.POSITIVE_INFINITY,
    gcTime: SESSION_QUERY_GC_MS,
  };
}

export function prefetchSessionConversation(
  queryClient: QueryClient,
  sessionId: string,
  isRunning = false,
) {
  void queryClient.prefetchQuery(transcriptQueryOptions(sessionId, isRunning));
  void queryClient.prefetchQuery(sessionArtifactsQueryOptions(sessionId, isRunning));
}
