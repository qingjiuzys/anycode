import { API_BASE, get } from "../http";

const TRANSCRIBE_TIMEOUT_MS = 120_000;

export interface MediaStatus {
  stt_configured: boolean;
  stt_provider?: string | null;
  stt_model?: string | null;
  stt_builtin?: boolean;
}

export interface TranscribeResult {
  ok: boolean;
  text?: string;
  error?: string;
  provider?: string;
  model?: string;
}

export const mediaClient = {
  mediaStatus: () => get<MediaStatus>("/api/media/status"),

  transcribeAudio: async (file: Blob, filename: string): Promise<TranscribeResult> => {
    const form = new FormData();
    form.append("file", file, filename);
    form.append("filename", filename);
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), TRANSCRIBE_TIMEOUT_MS);
    try {
      const url =
        API_BASE !== ""
          ? new URL("/api/media/transcribe", API_BASE.endsWith("/") ? API_BASE : `${API_BASE}/`).href
          : "/api/media/transcribe";
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
