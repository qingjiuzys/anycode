import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { api } from "@/api/client";

type TerminalServerMessage =
  | { type: "output"; data: string }
  | { type: "exit"; code: number }
  | { type: "error"; message: string };

export function useWorkbenchTerminal(
  projectId: string | null | undefined,
  active: boolean,
) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    if (!active || !projectId || !containerRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 12,
      fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
      theme: {
        background: "transparent",
      },
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());
    term.open(containerRef.current);
    fit.fit();
    termRef.current = term;

    const ws = new WebSocket(api.terminalWsUrl(projectId));
    wsRef.current = ws;

    ws.onopen = () => {
      term.writeln("\x1b[90mWorkbench terminal connected\x1b[0m");
    };

    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(String(ev.data)) as TerminalServerMessage;
        if (msg.type === "output") term.write(msg.data);
        if (msg.type === "error") term.writeln(`\r\n\x1b[31m${msg.message}\x1b[0m`);
        if (msg.type === "exit") term.writeln(`\r\n\x1b[90m[exit ${msg.code}]\x1b[0m`);
      } catch {
        term.write(String(ev.data));
      }
    };

    const dataDisp = term.onData((data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "input", data }));
      }
    });

    const resizeObs = new ResizeObserver(() => {
      fit.fit();
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(
          JSON.stringify({
            type: "resize",
            cols: term.cols,
            rows: term.rows,
          }),
        );
      }
    });
    resizeObs.observe(containerRef.current);

    return () => {
      dataDisp.dispose();
      resizeObs.disconnect();
      ws.close();
      term.dispose();
      termRef.current = null;
      wsRef.current = null;
    };
  }, [active, projectId]);

  return { containerRef };
}
