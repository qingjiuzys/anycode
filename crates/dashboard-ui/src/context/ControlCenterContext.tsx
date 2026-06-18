import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

type ControlCenterContextValue = {
  open: boolean;
  activePath: string;
  openControlCenter: (path?: string) => void;
  closeControlCenter: () => void;
  setActivePath: (path: string) => void;
};

const ControlCenterContext = createContext<ControlCenterContextValue | null>(null);

const DEFAULT_PATH = "/settings";

export function ControlCenterProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [activePath, setActivePath] = useState(DEFAULT_PATH);

  const openControlCenter = useCallback((path?: string) => {
    if (path) setActivePath(path);
    setOpen(true);
  }, []);

  const closeControlCenter = useCallback(() => {
    setOpen(false);
  }, []);

  const value = useMemo(
    () => ({
      open,
      activePath,
      openControlCenter,
      closeControlCenter,
      setActivePath,
    }),
    [open, activePath, openControlCenter, closeControlCenter],
  );

  return (
    <ControlCenterContext.Provider value={value}>{children}</ControlCenterContext.Provider>
  );
}

export function useControlCenter(): ControlCenterContextValue {
  const ctx = useContext(ControlCenterContext);
  if (!ctx) {
    throw new Error("useControlCenter must be used within ControlCenterProvider");
  }
  return ctx;
}
