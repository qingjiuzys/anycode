import { useCallback, useState } from "react";
import { copyImageToClipboard } from "@/lib/clipboardImage";

export function useClipboard() {
  const [copied, setCopied] = useState(false);
  const [copiedImage, setCopiedImage] = useState(false);

  const flash = useCallback((setter: (v: boolean) => void) => {
    setter(true);
    setTimeout(() => setter(false), 2000);
  }, []);

  const copy = useCallback(
    async (text: string) => {
      try {
        await navigator.clipboard.writeText(text);
        flash(setCopied);
        return true;
      } catch {
        return false;
      }
    },
    [flash],
  );

  const copyImage = useCallback(
    async (source: string | Blob) => {
      const ok = await copyImageToClipboard(source);
      if (ok) flash(setCopiedImage);
      return ok;
    },
    [flash],
  );

  return { copy, copyImage, copied, copiedImage };
}
