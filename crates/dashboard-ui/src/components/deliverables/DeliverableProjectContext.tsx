import { createContext, useContext, type ReactNode } from "react";

type DeliverableProjectValue = {
  projectId?: string;
  projectRoot?: string | null;
};

const DeliverableProjectContext = createContext<DeliverableProjectValue>({});

export function DeliverableProjectProvider({
  projectId,
  projectRoot,
  children,
}: DeliverableProjectValue & { children: ReactNode }) {
  return (
    <DeliverableProjectContext.Provider value={{ projectId, projectRoot }}>
      {children}
    </DeliverableProjectContext.Provider>
  );
}

export function useDeliverableProject(): DeliverableProjectValue {
  return useContext(DeliverableProjectContext);
}
