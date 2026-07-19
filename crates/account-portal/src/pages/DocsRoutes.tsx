import { useEffect } from "react";
import { Navigate, useLocation, useNavigate } from "react-router-dom";
import { DocsHelp } from "../docs/DocsHelp";
import { DocsHome } from "../docs/DocsHome";
import { DocsLayout } from "../docs/DocsLayout";
import { DocsPage } from "../docs/DocsPage";
import { parseDocsSlug } from "../docs/catalog";
import { useLocale } from "../i18n/context";

export function DocsRoutes() {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const locale = useLocale();
  const slug = parseDocsSlug(pathname);

  // Legacy bookmarks: /docs/zh/... → /docs/...
  useEffect(() => {
    if (!pathname.startsWith("/docs/zh")) return;
    const next = pathname === "/docs/zh" || pathname === "/docs/zh/" ? "/docs" : `/docs/${pathname.slice("/docs/zh/".length)}`;
    navigate(next, { replace: true });
  }, [pathname, navigate]);

  if (pathname.startsWith("/docs/zh")) {
    return <Navigate to={pathname === "/docs/zh" || pathname === "/docs/zh/" ? "/docs" : `/docs/${pathname.slice("/docs/zh/".length)}`} replace />;
  }

  let content;
  if (slug === "help") {
    content = <DocsHelp locale={locale} />;
  } else if (!slug) {
    content = <DocsHome locale={locale} />;
  } else if (slug.startsWith("guide/")) {
    content = <DocsPage locale={locale} slug={slug} />;
  } else {
    content = <DocsHome locale={locale} />;
  }

  return <DocsLayout locale={locale}>{content}</DocsLayout>;
}
