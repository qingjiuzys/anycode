import { createContext, useContext, type ReactNode } from "react";

const EmbeddedControlCenterContext = createContext(false);

export function EmbeddedControlCenterProvider({
  children,
  value = true,
}: {
  children: ReactNode;
  value?: boolean;
}) {
  return (
    <EmbeddedControlCenterContext.Provider value={value}>
      {children}
    </EmbeddedControlCenterContext.Provider>
  );
}

export function useEmbeddedControlCenter(): boolean {
  return useContext(EmbeddedControlCenterContext);
}
