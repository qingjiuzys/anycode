import { useState } from "react";
import { Icon } from "@/components/Icon";
import { useT } from "@/i18n/context";
import { appleOcrImage, isTauriDesktop } from "@/lib/desktopShell";
import { mergeVoiceTranscript } from "@/components/VoiceInputButton";

type ImagePayload = {
  mime_type: string;
  data_base64: string;
};

type Props = {
  images: ImagePayload[];
  disabled?: boolean;
  onText: (text: string) => void;
};

export function ImageOcrButton({ images, disabled, onText }: Props) {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!isTauriDesktop() || images.length === 0) {
    return null;
  }

  const handleClick = async () => {
    if (disabled || busy) return;
    setError(null);
    setBusy(true);
    try {
      const parts: string[] = [];
      for (const img of images) {
        const result = await appleOcrImage(img.mime_type, img.data_base64);
        if (!result.ok) {
          setError(result.error);
          setBusy(false);
          return;
        }
        if (result.text.trim()) {
          parts.push(result.text.trim());
        }
      }
      const merged = parts.join("\n\n");
      if (!merged) {
        setError(t("conversations.ocrEmpty"));
        setBusy(false);
        return;
      }
      onText(merged);
    } catch (e) {
      setError(e instanceof Error ? e.message : t("conversations.ocrError"));
    } finally {
      setBusy(false);
    }
  };

  const title = busy ? t("conversations.ocrExtracting") : t("conversations.ocrExtract");

  return (
    <div className="flex flex-col items-start gap-1">
      <button
        type="button"
        className="dw-btn-secondary text-xs py-1"
        disabled={disabled || busy}
        title={title}
        aria-label={title}
        onClick={() => void handleClick()}
      >
        {busy ? <Icon name="hourglass_empty" size={14} /> : <Icon name="document_scanner" size={14} />}
        <span className="ml-1 hidden sm:inline">{t("conversations.ocrExtract")}</span>
      </button>
      {error && (
        <span className="text-[11px] text-error max-w-[14rem] leading-snug">{error}</span>
      )}
    </div>
  );
}

export function appendOcrToMessage(prev: string, ocrText: string): string {
  return mergeVoiceTranscript(prev, ocrText);
}
