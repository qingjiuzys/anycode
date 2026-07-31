import { useRef, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useMediaStatus } from "@/hooks/useMediaStatus";
import { useT } from "@/i18n/context";

/** Speak assistant text via the active TTS capability slot (not the chat brain). */
export function SpeakButton({ text }: { text: string }) {
  const t = useT();
  const { ttsAvailable, isLoading } = useMediaStatus();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const trimmed = text.trim();
  if (!trimmed) return null;

  const title = !ttsAvailable
    ? t("conversations.ttsUnavailable")
    : busy
      ? t("conversations.ttsSpeaking")
      : error ?? t("conversations.ttsSpeak");

  return (
    <button
      type="button"
      className="dw-btn-ghost text-[10px] py-0.5"
      disabled={isLoading || !ttsAvailable || busy}
      title={title}
      aria-label={t("conversations.ttsSpeak")}
      onClick={async () => {
        setError(null);
        setBusy(true);
        try {
          audioRef.current?.pause();
          const result = await api.synthesizeSpeech(trimmed.slice(0, 4000));
          if (!result.ok || !result.audio_base64) {
            setError(result.error ?? t("conversations.ttsError"));
            return;
          }
          const mime = result.mime_type || "audio/mpeg";
          const src = `data:${mime};base64,${result.audio_base64}`;
          const audio = new Audio(src);
          audioRef.current = audio;
          await audio.play();
        } catch (e) {
          setError(e instanceof Error ? e.message : t("conversations.ttsError"));
        } finally {
          setBusy(false);
        }
      }}
    >
      <Icon name={busy ? "hourglass_empty" : "play_arrow"} size={12} />
    </button>
  );
}
