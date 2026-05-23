import {
  createContext,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type AuthUser } from "@/api/client";

interface AuthContextValue {
  user: AuthUser | null;
  authenticated: boolean;
  loading: boolean;
  requiresLogin: boolean;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refetch: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const me = useQuery({
    queryKey: ["auth-me"],
    queryFn: api.authMe,
    retry: false,
    staleTime: 60_000,
  });

  const loginMut = useMutation({
    mutationFn: ({ email, password }: { email: string; password: string }) =>
      api.login(email, password),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["auth-me"] }),
  });

  const logoutMut = useMutation({
    mutationFn: api.logout,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["auth-me"] }),
  });

  const value = useMemo<AuthContextValue>(() => {
    const authenticated = me.data?.authenticated ?? false;
    const user = me.data?.user ?? null;
    return {
      user,
      authenticated,
      loading: me.isLoading,
      requiresLogin: !me.isLoading && !authenticated && me.isError,
      login: async (email, password) => {
        await loginMut.mutateAsync({ email, password });
      },
      logout: async () => {
        await logoutMut.mutateAsync();
      },
      refetch: () => void me.refetch(),
    };
  }, [me.data, me.isLoading, me.isError, loginMut, logoutMut, me]);

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth outside provider");
  return ctx;
}
