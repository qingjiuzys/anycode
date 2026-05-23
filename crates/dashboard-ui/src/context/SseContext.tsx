import { createContext, useContext, type ReactNode } from "react";
import { useGlobalEventStream } from "@/hooks/useGlobalEventStream";
import type { SseStatus } from "@/hooks/useEventSource";

const SseContext = createContext<SseStatus>("offline");

export function SseProvider({ children }: { children: ReactNode }) {
  const status = useGlobalEventStream();
  return <SseContext.Provider value={status}>{children}</SseContext.Provider>;
}

export function useSseStatus(): SseStatus {
  return useContext(SseContext);
}
