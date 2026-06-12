import { useEffect, useRef, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useMediaStatus } from "@/hooks/useMediaStatus";
import { useT } from "@/i18n/context";
import {
  blobToWav16k,
  extensionForMime,
  isVoiceRecordingSupported,
  startRecording,
  stopRecording,
  VoiceRecordingError,
} from "@/lib/voiceRecording";

type Props = {
  onTranscribed: (text: string) => void;
  disabled?: boolean;
  className?: string;
};

type Phase = "idle" | "recording" | "transcribing";

function appendText(prev: string, next: string): string {
  const t = next.trim();
  if (!t) return prev;
  if (!prev.trim()) return t;
  return `${prev.trimEnd()} ${t}`;
}

export function VoiceInputButton({ onTranscribed, disabled, className }: Props) {
  const t = useT();
  const { sttAvailable, sttBuiltin, isLoading } = useMediaStatus();
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [seconds, setSeconds] = useState(0);
  const timerRef = useRef<number | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearInterval(timerRef.current);
      }
    };
  }, []);

  if (isLoading || !sttAvailable || !isVoiceRecordingSupported()) {
    return null;
  }

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
    setSeconds(0);
  };

  const handleClick = async () => {
    if (disabled || phase === "transcribing") return;
    setError(null);

    if (phase === "recording") {
      clearTimer();
      setPhase("transcribing");
      try {
        const blob = await stopRecording();
        let upload = blob;
        let filename = `recording.${extensionForMime(blob.type)}`;
        if (sttBuiltin) {
          upload = await blobToWav16k(blob);
          filename = "recording.wav";
        }
        const result = await api.transcribeAudio(upload, filename);
        if (!result.ok || !result.text?.trim()) {
          setError(result.error ?? t("settings.model.voiceInput.error"));
          setPhase("idle");
          return;
        }
        onTranscribed(result.text.trim());
        setPhase("idle");
      } catch (e) {
        if (e instanceof VoiceRecordingError) {
          if (e.code === "permission_denied") {
            setError(t("settings.model.voiceInput.permissionDenied"));
          } else if (e.code === "empty_recording") {
            setError(t("settings.model.voiceInput.empty"));
          } else {
            setError(e.message);
          }
        } else {
          setError(e instanceof Error ? e.message : t("settings.model.voiceInput.error"));
        }
        setPhase("idle");
      }
      return;
    }

    try {
      await startRecording();
      setPhase("recording");
      setSeconds(0);
      timerRef.current = window.setInterval(() => {
        setSeconds((s) => s + 1);
      }, 1000);
    } catch (e) {
      if (e instanceof VoiceRecordingError && e.code === "permission_denied") {
        setError(t("settings.model.voiceInput.permissionDenied"));
      } else {
        setError(e instanceof Error ? e.message : t("settings.model.voiceInput.error"));
      }
    }
  };

  const title =
    phase === "recording"
      ? t("settings.model.voiceInput.stop")
      : phase === "transcribing"
        ? t("settings.model.voiceInput.transcribing")
        : t("settings.model.voiceInput.start");

  return (
    <div className={`flex flex-col items-start gap-1 ${className ?? ""}`}>
      <button
        type="button"
        className={`dw-btn-secondary text-xs py-1 ${phase === "recording" ? "text-error border-error/40" : ""}`}
        disabled={disabled || phase === "transcribing"}
        title={title}
        aria-label={title}
        onClick={() => void handleClick()}
      >
        {phase === "transcribing" ? (
          <Icon name="hourglass_empty" size={14} />
        ) : phase === "recording" ? (
          <Icon name="stop" size={14} />
        ) : (
          <Icon name="mic" size={14} />
        )}
        {phase === "recording" && (
          <span className="ml-1 tabular-nums">{seconds}s</span>
        )}
      </button>
      {error && (
        <span className="text-[11px] text-error max-w-[14rem] leading-snug">{error}</span>
      )}
    </div>
  );
}

/** Helper for composers that manage prompt state. */
export function mergeVoiceTranscript(prev: string, transcript: string): string {
  return appendText(prev, transcript);
}
