import { Link } from "@tanstack/react-router";
import type { SearchResults } from "@/api/types";
import { useT } from "@/i18n/context";
import { localizeLogTitle } from "@/lib/eventFormat";

export function SearchResultsDropdown({
  data,
  className = "",
}: {
  data: SearchResults;
  className?: string;
}) {
  const t = useT();
  const hasProjects = (data.projects ?? []).length > 0;
  const hasSessions = (data.sessions ?? []).length > 0;
  const hasEvents = (data.events ?? []).length > 0;

  if (!hasProjects && !hasSessions && !hasEvents) {
    return <p className="px-3 py-2 text-xs text-secondary m-0">{t("search.noResults")}</p>;
  }

  return (
    <>
      {(data.projects ?? []).map((p) => (
        <Link
          key={p.id}
          to="/projects/$projectId"
          params={{ projectId: p.id }}
          className={`block px-3 py-2 text-sm hover:bg-surface-container no-underline ${className}`}
        >
          <span className="text-secondary text-xs">{t("nav.projects")}</span> {p.title}
        </Link>
      ))}
      {(data.sessions ?? []).map((s) => (
        <Link
          key={s.id}
          to="/sessions/$sessionId"
          params={{ sessionId: s.id }}
          className={`block px-3 py-2 text-sm hover:bg-surface-container no-underline ${className}`}
        >
          <span className="text-secondary text-xs">{t("nav.conversations")}</span> {s.title}
        </Link>
      ))}
      {(data.events ?? []).map((e) => (
        <Link
          key={e.id}
          to="/events/$eventId"
          params={{ eventId: e.id }}
          className={`block px-3 py-2 text-sm hover:bg-surface-container no-underline ${className}`}
        >
          <span className="text-secondary text-xs">{t("search.events")}</span>{" "}
          {localizeLogTitle(e.title, "", t) ?? e.title}
        </Link>
      ))}
    </>
  );
}
