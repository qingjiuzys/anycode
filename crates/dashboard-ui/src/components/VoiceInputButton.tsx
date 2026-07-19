import { useEffect, useRef, useState } from "react";
import { api } from "@/api/client";
import { Icon } from "@/components/Icon";
import { useMediaStatus } from "@/hooks/useMediaStatus";
import { useT } from "@/i18n/context";
import {
  appleTranscribeAudio,
  isTauriDesktop,
  shouldUseNativeAppleSpeech,
} from "@/lib/desktopShell";
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

function VoiceWaveBars({ active }: { active: boolean }) {
  return (
    <span
      className={`dw-voice-wave ${active ? "dw-voice-wave--active" : "dw-voice-wave--idle"}`}
      aria-hidden
    >
      {Array.from({ length: 5 }, (_, i) => (
        <span key={i} className="dw-voice-wave__bar" />
      ))}
    </span>
  );
}

function appendText(prev: string, next: string): string {
  const t = next.trim();
  if (!t) return prev;
  if (!prev.trim()) return t;
  return `${prev.trimEnd()} ${t}`;
}

export function VoiceInputButton({ onTranscribed, disabled, className }: Props) {
  const t = useT();
  const { sttAvailable, sttBuiltin, sttProvider, appleMedia, isLoading } = useMediaStatus();
  const [phase, setPhase] = useState<Phase>("idle");
  const [error, setError] = useState<string | null>(null);
  const [seconds, setSeconds] = useState(0);
  const timerRef = useRef<number | null>(null);
  const nativeAppleSpeech = shouldUseNativeAppleSpeech(sttProvider, appleMedia?.stt);

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) {
        window.clearInterval(timerRef.current);
      }
    };
  }, []);

  if (isLoading || !sttAvailable) {
    return null;
  }

  const recordingSupported = isVoiceRecordingSupported();

  const clearTimer = () => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
    setSeconds(0);
  };

  const handleClick = async () => {
    if (disabled || phase === "transcribing") return;
    if (!recordingSupported) {
      setError(t("settings.model.voiceInput.unsupported"));
      return;
    }
    if (nativeAppleSpeech && !isTauriDesktop()) {
      setError(t("settings.model.voiceInput.desktopOnly"));
      return;
    }
    setError(null);

    if (phase === "recording") {
      clearTimer();
      setPhase("transcribing");
      try {
        const blob = await stopRecording();
        if (nativeAppleSpeech) {
          const wav = await blobToWav16k(blob);
          const result = await appleTranscribeAudio(wav);
          if (!result.ok) {
            const err = result.error.toLowerCase();
            const msg =
              result.error === "not_desktop"
                ? t("settings.model.voiceInput.desktopOnly")
                : result.error === "apple_media_timeout"
                  ? t("settings.model.voiceInput.timeout")
                  : err.includes("no speech detected") || err.includes("empty transcription")
                    ? t("settings.model.voiceInput.empty")
                    : result.error;
            setError(msg);
            setPhase("idle");
            return;
          }
          if (!result.text.trim()) {
            setError(t("settings.model.voiceInput.empty"));
            setPhase("idle");
            return;
          }
          onTranscribed(result.text.trim());
          setPhase("idle");
          return;
        }

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

  const buttonClass =
    phase === "recording"
      ? "dw-voice-input-btn dw-voice-input-btn--recording"
      : phase === "transcribing"
        ? "dw-voice-input-btn dw-voice-input-btn--transcribing"
        : "dw-voice-input-btn";

  return (
    <div className={`flex flex-col items-start gap-1 ${className ?? ""}`}>
      <div className="flex items-center gap-1.5">
        <button
          type="button"
          className={buttonClass}
          disabled={disabled || phase === "transcribing"}
          title={title}
          aria-label={title}
          aria-pressed={phase === "recording" ? true : undefined}
          onClick={() => void handleClick()}
        >
          {phase === "transcribing" ? (
            <Icon name="hourglass_empty" size={16} className="dw-voice-input-btn__hourglass" />
          ) : phase === "recording" ? (
            <VoiceWaveBars active />
          ) : (
            <>
              <Icon name="mic" size={16} className="dw-voice-input-btn__mic" />
              <VoiceWaveBars active={false} />
            </>
          )}
        </button>
        {phase === "recording" && (
          <span className="text-[11px] font-medium text-primary tabular-nums">{seconds}s</span>
        )}
      </div>
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
