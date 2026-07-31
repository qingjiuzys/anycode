import { get, apiUrl } from "../http";

const TRANSCRIBE_TIMEOUT_MS = 120_000;

export interface MediaStatus {
  stt_configured: boolean;
  stt_provider?: string | null;
  stt_model?: string | null;
  stt_builtin?: boolean;
  tts_configured?: boolean;
  tts_provider?: string | null;
  tts_model?: string | null;
  ocr_available?: boolean;
  chat_supports_vision?: boolean;
  /** Chat has vision OR OCR fallback — safe to attach images. */
  image_attach_ok?: boolean;
  apple_media?: AppleMediaCapabilities | null;
}

export interface AppleMediaCapabilities {
  stt: boolean;
  ocr: boolean;
  tts: boolean;
  notify: boolean;
  keychain: boolean;
  pasteboard: boolean;
  platform: string;
  helper_path?: string | null;
  speech_authorized?: boolean | null;
  microphone_authorized?: boolean | null;
}

export interface TranscribeResult {
  ok: boolean;
  text?: string;
  error?: string;
  provider?: string;
  model?: string;
}

export interface OcrResult {
  ok: boolean;
  text?: string;
  error?: string;
  provider?: string;
}

export interface TtsResult {
  ok: boolean;
  audio_base64?: string;
  mime_type?: string;
  error?: string;
  provider?: string;
  model?: string;
}

export const mediaClient = {
  mediaStatus: () => get<MediaStatus>("/api/media/status"),

  ocrImages: async (
    images: { mime_type: string; data_base64: string }[],
  ): Promise<OcrResult> => {
    try {
      const url = apiUrl("/api/media/ocr");
      const res = await fetch(url, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ images }),
      });
      const data = (await res.json()) as OcrResult;
      if (!res.ok) {
        return { ok: false, error: data.error ?? `${res.status} ocr failed` };
      }
      return data;
    } catch (e) {
      return { ok: false, error: e instanceof Error ? e.message : String(e) };
    }
  },

  synthesizeSpeech: async (text: string): Promise<TtsResult> => {
    try {
      const url = apiUrl("/api/media/tts");
      const res = await fetch(url, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text }),
      });
      const data = (await res.json()) as TtsResult;
      if (!res.ok) {
        return { ok: false, error: data.error ?? `${res.status} tts failed` };
      }
      return data;
    } catch (e) {
      return { ok: false, error: e instanceof Error ? e.message : String(e) };
    }
  },

  transcribeAudio: async (file: Blob, filename: string): Promise<TranscribeResult> => {
    const form = new FormData();
    form.append("file", file, filename);
    form.append("filename", filename);
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), TRANSCRIBE_TIMEOUT_MS);
    try {
      const url = apiUrl("/api/media/transcribe");
      const res = await fetch(url, {
        method: "POST",
        credentials: "include",
        body: form,
        signal: controller.signal,
      });
      const data = (await res.json()) as TranscribeResult & { error?: string };
      if (!res.ok) {
        return { ok: false, error: data.error ?? `${res.status} transcribe failed` };
      }
      return data;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      return { ok: false, error: msg };
    } finally {
      clearTimeout(timer);
    }
  },
};
