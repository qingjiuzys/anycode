import { useLayoutEffect, useState, type CSSProperties, type RefObject } from "react";

/** Fixed popup above an anchor — avoids overflow:hidden clipping in composers. */
export function useAnchoredAboveStyle(
  open: boolean,
  anchorRef: RefObject<HTMLElement | null>,
  opts?: { matchWidth?: boolean; minWidth?: number; maxWidth?: number },
): CSSProperties {
  const [style, setStyle] = useState<CSSProperties>({});
  const matchWidth = opts?.matchWidth ?? false;
  const minWidth = opts?.minWidth ?? 0;
  const maxWidth = opts?.maxWidth ?? 384;

  useLayoutEffect(() => {
    if (!open || !anchorRef.current) return;
    const update = () => {
      const rect = anchorRef.current!.getBoundingClientRect();
      const width = matchWidth
        ? Math.min(rect.width, window.innerWidth - 16)
        : Math.min(Math.max(rect.width, minWidth), maxWidth, window.innerWidth - 16);
      const left = Math.max(8, Math.min(rect.left, window.innerWidth - width - 8));
      setStyle({
        position: "fixed",
        left,
        bottom: window.innerHeight - rect.top + 8,
        width,
        zIndex: 300,
      });
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
  }, [open, anchorRef, matchWidth, minWidth, maxWidth]);

  return style;
}
