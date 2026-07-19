import { useEffect, useRef, useState } from "react";
import { smoothTextStep } from "@/lib/smoothTextCore";

export type SmoothTextResult = {
  text: string;
  /** True while catching up after stream ended but displayed text is still short of target. */
  isRevealing: boolean;
};

/**
 * Reveal streaming text at a fixed rate. Keeps animating after `streamActive`
 * goes false until displayed text catches up to target.
 */
export function useSmoothText(
  streamKey: string,
  target: string,
  streamActive: boolean,
): SmoothTextResult {
  const [displayed, setDisplayed] = useState(target);
  const displayedRef = useRef(target);
  const targetRef = useRef(target);
  const keyRef = useRef(streamKey);
  const streamActiveRef = useRef(streamActive);
  const hasStreamedRef = useRef(false);
  const rafRef = useRef<number | null>(null);
  const lastTsRef = useRef<number | null>(null);

  const cancelLoop = () => {
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
  };

  const isAnimating = () => {
    if (streamActiveRef.current) {
      return true;
    }
    return (
      hasStreamedRef.current &&
      displayedRef.current.length < targetRef.current.length
    );
  };

  const scheduleTick = () => {
    if (rafRef.current !== null) return;
    const tick = (ts: number) => {
      if (!isAnimating()) {
        rafRef.current = null;
        lastTsRef.current = null;
        return;
      }
      const last = lastTsRef.current ?? ts;
      const deltaMs = ts - last;
      lastTsRef.current = ts;
      const next = smoothTextStep(
        displayedRef.current,
        targetRef.current,
        deltaMs,
      );
      if (next !== displayedRef.current) {
        displayedRef.current = next;
        setDisplayed(next);
      }
      if (next.length < targetRef.current.length) {
        rafRef.current = requestAnimationFrame(tick);
      } else {
        rafRef.current = null;
        lastTsRef.current = null;
      }
    };
    rafRef.current = requestAnimationFrame(tick);
  };

  useEffect(() => {
    targetRef.current = target;
    if (isAnimating() && displayedRef.current.length < target.length) {
      scheduleTick();
    }
  }, [target]);

  useEffect(() => {
    streamActiveRef.current = streamActive;
    targetRef.current = target;

    if (keyRef.current !== streamKey) {
      keyRef.current = streamKey;
      hasStreamedRef.current = false;
      displayedRef.current = streamActive ? "" : target;
      setDisplayed(streamActive ? "" : target);
      lastTsRef.current = null;
      cancelLoop();
    }

    if (streamActive) {
      hasStreamedRef.current = true;
      if (displayedRef.current.length < targetRef.current.length) {
        scheduleTick();
      }
      return cancelLoop;
    }

    if (
      hasStreamedRef.current &&
      displayedRef.current.length < targetRef.current.length
    ) {
      scheduleTick();
      return cancelLoop;
    }

    cancelLoop();
    displayedRef.current = target;
    setDisplayed(target);
    lastTsRef.current = null;
  }, [streamKey, streamActive, target]);

  const animating = streamActive
    ? true
    : hasStreamedRef.current && displayed.length < target.length;
  const isRevealing = !streamActive && animating;

  return {
    text: animating ? displayed : target,
    isRevealing,
  };
}
