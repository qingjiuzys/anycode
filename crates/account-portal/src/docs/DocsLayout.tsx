import { Link, useLocation } from "react-router-dom";
import type { ReactNode } from "react";
import { docsBase, docsPageHref, docsSections, type DocsLocale } from "./catalog";

export function DocsLayout({ locale, children }: { locale: DocsLocale; children: ReactNode }) {
  const loc = useLocation();
  const base = docsBase();
  const sections = docsSections(locale);

  return (
    <div className="docs-shell nx-docs">
      <aside className="docs-sidebar" aria-label={locale === "zh" ? "文档导航" : "Documentation navigation"}>
        <div className="docs-sidebar__head">
          <Link className="docs-sidebar__home" to={base}>
            {locale === "zh" ? "anyCode 文档" : "anyCode Docs"}
          </Link>
        </div>
        {sections.map((section) => (
          <div key={section.text} className="docs-sidebar__section">
            <p className="docs-sidebar__section-title">{section.text}</p>
            <ul>
              {section.items.map((item) => {
                const href = docsPageHref(locale, item.slug);
                const active = loc.pathname === href || loc.pathname === `${href}/`;
                return (
                  <li key={item.slug}>
                    <Link className={active ? "active" : undefined} to={href}>
                      {item.text}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
        <div className="docs-sidebar__section">
          <ul>
            <li>
              <Link
                className={loc.pathname === "/docs/help" || loc.pathname === "/docs/help/" ? "active" : undefined}
                to="/docs/help"
              >
                {locale === "zh" ? "帮助与支持" : "Help & support"}
              </Link>
            </li>
          </ul>
        </div>
      </aside>
      <div className="docs-main">{children}</div>
    </div>
  );
}
