import { memo, useMemo } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import "highlight.js/styles/github.css";
import { MermaidDiagram } from "@/components/chat/MermaidDiagram";
import { TableCard } from "@/components/chat/TableCard";
import { splitMarkdownWithTables } from "@/lib/markdownTable";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("python", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("html", xml);

type Props = {
  text: string;
  className?: string;
  /** While streaming, render plain text; parse markdown once settled. */
  live?: boolean;
};

function MarkdownBlock({
  content,
  components,
}: {
  content: string;
  components: React.ComponentProps<typeof ReactMarkdown>["components"];
}) {
  if (!content.trim()) return null;
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
      {content}
    </ReactMarkdown>
  );
}

export const TranscriptMarkdown = memo(function TranscriptMarkdown({
  text,
  className = "",
  live = false,
}: Props) {
  const displayText = useMemo(
    () => text.replace(/^[ \t]*(\*{3,}|-{3,}|_{3,})[ \t]*$/gm, ""),
    [text],
  );
  const components = useMemo(
    () => ({
      code({ className: codeClass, children, ...props }: React.ComponentProps<"code">) {
        const match = /language-(\w+)/.exec(codeClass ?? "");
        const raw = String(children).replace(/\n$/, "");
        if (match?.[1] === "mermaid") {
          return <MermaidDiagram code={raw} />;
        }
        if (match) {
          const lang = match[1];
          let highlighted = raw;
          try {
            if (hljs.getLanguage(lang)) {
              highlighted = hljs.highlight(raw, { language: lang }).value;
            }
          } catch {
            /* keep raw */
          }
          return (
            <pre className="dw-transcript-code">
              <code
                className={codeClass}
                dangerouslySetInnerHTML={{ __html: highlighted }}
                {...props}
              />
            </pre>
          );
        }
        if (raw.includes("\n")) {
          return (
            <pre className="dw-transcript-code">
              <code {...props}>{raw}</code>
            </pre>
          );
        }
        return (
          <code className="dw-transcript-inline-code" {...props}>
            {children}
          </code>
        );
      },
      a({ href, children, ...props }: React.ComponentProps<"a">) {
        return (
          <a href={href} target="_blank" rel="noreferrer" {...props}>
            {children}
          </a>
        );
      },
    }),
    [],
  );

  const segments = useMemo(
    () => (live ? [{ type: "markdown" as const, content: displayText }] : splitMarkdownWithTables(displayText)),
    [displayText, live],
  );

  if (live) {
    return (
      <div className={`dw-transcript-markdown ${className}`}>
        <div className="whitespace-pre-wrap break-words leading-relaxed">{displayText}</div>
      </div>
    );
  }

  return (
    <div className={`dw-transcript-markdown ${className}`}>
      {segments.map((segment, index) =>
        segment.type === "table" ? (
          <TableCard key={`table-${index}`} table={segment.table} />
        ) : (
          <MarkdownBlock key={`md-${index}`} content={segment.content} components={components} />
        ),
      )}
    </div>
  );
});
