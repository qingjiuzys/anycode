import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Link } from "react-router-dom";
import type { DocsLocale } from "./catalog";

type Props = {
  locale: DocsLocale;
  slug: string;
};

function sourcePath(locale: DocsLocale, slug: string): string {
  const prefix = locale === "zh" ? "/docs-src/zh" : "/docs-src/en";
  return `${prefix}/${slug}.md`;
}

export function DocsPage({ locale, slug }: Props) {
  const [markdown, setMarkdown] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setMarkdown(null);
    setError(null);
    fetch(sourcePath(locale, slug))
      .then((res) => {
        if (!res.ok) throw new Error(`${res.status}`);
        return res.text();
      })
      .then((text) => {
        if (!cancelled) setMarkdown(text);
      })
      .catch(() => {
        if (!cancelled) setError(locale === "zh" ? "未找到该文档页。" : "Document page not found.");
      });
    return () => {
      cancelled = true;
    };
  }, [locale, slug]);

  if (error) {
    return (
      <div className="docs-article">
        <p>{error}</p>
        <Link to="/docs">{locale === "zh" ? "返回文档首页" : "Back to docs home"}</Link>
      </div>
    );
  }

  if (!markdown) {
    return <div className="docs-article docs-article--loading">{locale === "zh" ? "加载中…" : "Loading…"}</div>;
  }

  return (
    <article className="docs-article">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>
    </article>
  );
}
