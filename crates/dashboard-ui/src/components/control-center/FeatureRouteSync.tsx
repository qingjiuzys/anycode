import { useEffect, useRef } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { useControlCenter } from "@/context/ControlCenterContext";
import {
  controlCenterHref,
  shouldOpenControlCenterForLocation,
} from "@/lib/controlCenterPaths";
import { conversationSearchParams, parseConversationSearch } from "@/lib/conversationsSearch";

/** Deep-link feature URLs into conversations + control center overlay. */
export function FeatureRouteSync() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const searchStr = useRouterState({ select: (s) => s.location.searchStr });
  const navigate = useNavigate();
  const { openControlCenter } = useControlCenter();
  const lastHandled = useRef<string | null>(null);

  useEffect(() => {
    const params = new URLSearchParams(searchStr.startsWith("?") ? searchStr.slice(1) : searchStr);
    const cc = params.get("cc");

    if (cc && pathname === "/conversations") {
      if (lastHandled.current === cc) return;
      lastHandled.current = cc;
      openControlCenter(cc);
      params.delete("cc");
      const rest = params.toString();
      const canon = conversationSearchParams(parseConversationSearch(rest ? `?${rest}` : ""));
      void navigate({
        to: "/conversations",
        search: () => canon,
        replace: true,
      });
      return;
    }

    if (!shouldOpenControlCenterForLocation(pathname, searchStr)) {
      lastHandled.current = null;
      return;
    }

    const href = controlCenterHref(pathname, searchStr);
    if (lastHandled.current === href) return;
    lastHandled.current = href;

    openControlCenter(href);
    if (pathname !== "/conversations") {
      void navigate({ to: "/conversations", search: { cc: href }, replace: true });
    }
  }, [pathname, searchStr, navigate, openControlCenter]);

  return null;
}
