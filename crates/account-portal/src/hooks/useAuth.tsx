import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api, getToken, setToken as persistToken, setUnauthorizedHandler } from "../api";

type AuthContextValue = {
  token: string | null;
  authenticated: boolean;
  validating: boolean;
  setToken: (token: string | null) => void;
  logout: () => void;
};

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setTokenState] = useState<string | null>(() => getToken());
  const [validating, setValidating] = useState(() => Boolean(getToken()));

  const logout = useCallback(() => {
    persistToken(null);
    setTokenState(null);
  }, []);

  const setToken = useCallback((next: string | null) => {
    persistToken(next);
    setTokenState(next);
    setValidating(false);
  }, []);

  useEffect(() => {
    setUnauthorizedHandler(() => {
      logout();
      const next = `/login?reason=session_expired`;
      if (window.location.pathname.startsWith("/console")) {
        window.location.assign(next);
      }
    });
    return () => setUnauthorizedHandler(null);
  }, [logout]);

  useEffect(() => {
    if (!token) {
      setValidating(false);
      return;
    }
    let cancelled = false;
    setValidating(true);
    void api
      .me()
      .then(() => {
        if (!cancelled) setValidating(false);
      })
      .catch(() => {
        if (!cancelled) {
          logout();
          setValidating(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, logout]);

  const value = useMemo(
    () => ({
      token,
      authenticated: Boolean(token) && !validating,
      validating,
      setToken,
      logout,
    }),
    [token, validating, setToken, logout],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used within AuthProvider");
  return ctx;
}
