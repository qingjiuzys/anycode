import { useEffect, useId, useRef, useState } from "react";
import { useT } from "@/i18n/context";

type Props = {
  code: string;
};

export function MermaidDiagram({ code }: Props) {
  const t = useT();
  const containerRef = useRef<HTMLDivElement>(null);
  const diagramId = useId().replace(/:/g, "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const render = async () => {
      try {
        const mermaid = await import("mermaid");
        mermaid.default.initialize({
          startOnLoad: false,
          theme: "neutral",
          securityLevel: "strict",
        });
        if (!containerRef.current || cancelled) return;
        const { svg } = await mermaid.default.render(`mmd-${diagramId}`, code);
        if (!cancelled && containerRef.current) {
          containerRef.current.innerHTML = svg;
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    };
    void render();
    return () => {
      cancelled = true;
    };
  }, [code, diagramId]);

  if (error) {
    return (
      <pre className="dw-transcript-code">
        <code>{code}</code>
      </pre>
    );
  }

  return (
    <div className="dw-mermaid-card">
      <p className="dw-mermaid-card__label">{t("conversations.mermaid.label")}</p>
      <div ref={containerRef} className="dw-mermaid-card__canvas" />
    </div>
  );
}
