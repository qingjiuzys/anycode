import { useCallback, useRef } from "react";

/**
 * IME (input method) guard for composer textareas.
 * While composing, Enter must go to the IME — not submit the message.
 */
export function useComposerIme() {
  const composingRef = useRef(false);

  const compositionProps = {
    onCompositionStart: () => {
      composingRef.current = true;
    },
    onCompositionEnd: () => {
      // The Enter that confirms IME may still bubble as keydown; defer clearing.
      window.setTimeout(() => {
        composingRef.current = false;
      }, 0);
    },
  };

  const shouldIgnoreEnterForIme = useCallback((e: React.KeyboardEvent) => {
    if (composingRef.current) return true;
    const ne = e.nativeEvent;
    if (ne.isComposing) return true;
    // Legacy WebKit / some IME backends
    if (ne.keyCode === 229) return true;
    return false;
  }, []);

  return { compositionProps, shouldIgnoreEnterForIme };
}
