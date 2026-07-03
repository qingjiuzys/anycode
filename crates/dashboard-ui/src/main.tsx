import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { router } from "./router";
import { initDensity } from "./hooks/useDensity";
import { applySkin, getSkin } from "./hooks/useSkin";
import { getTheme, setTheme } from "./hooks/useTheme";
import { I18nProvider } from "./i18n/context";
import { AuthProvider } from "./auth/context";
import { AccountCloudProvider } from "./hooks/useAccountCloud";
import { SseProvider } from "./context/SseContext";
import { isTauriDesktop } from "./lib/desktopShell";
import "./index.css";

/** Tauri serves `index.html` at `/index.html`; TanStack Router expects `/`. */
function normalizeSpaPathname(): void {
  if (typeof window === "undefined") return;
  const { pathname, search, hash } = window.location;
  if (pathname.endsWith("/index.html")) {
    const base = pathname.slice(0, -"index.html".length) || "/";
    window.history.replaceState(null, "", `${base}${search}${hash}`);
  }
}

normalizeSpaPathname();

initDensity();
applySkin(getSkin());
setTheme(getTheme());

if (isTauriDesktop()) {
  document.documentElement.classList.add("dw-tauri");
}

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 5_000, retry: 1, gcTime: 30 * 60_000 },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <I18nProvider>
        <AuthProvider>
          <AccountCloudProvider>
            <SseProvider>
              <RouterProvider router={router} />
            </SseProvider>
          </AccountCloudProvider>
        </AuthProvider>
      </I18nProvider>
    </QueryClientProvider>
  </StrictMode>,
);
