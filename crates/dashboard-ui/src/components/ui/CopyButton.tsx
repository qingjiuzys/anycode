import { useClipboard } from "@/hooks/useClipboard";
import { useT } from "@/i18n/context";

export function CopyButton({ text, label }: { text: string; label?: string }) {
  const t = useT();
  const { copy, copied } = useClipboard();
  const displayLabel = label ?? t("common.copy");
  return (
    <button type="button" className="dw-btn-secondary" onClick={() => copy(text)}>
      {copied ? t("common.copied") : displayLabel}
    </button>
  );
}
