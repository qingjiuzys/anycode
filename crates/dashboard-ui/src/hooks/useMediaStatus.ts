import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";

export function useMediaStatus() {
  const query = useQuery({
    queryKey: ["media-status"],
    queryFn: () => api.mediaStatus(),
    staleTime: 60_000,
  });

  return {
    ...query,
    sttAvailable: query.data?.stt_configured === true,
    sttBuiltin: query.data?.stt_builtin === true,
    sttProvider: query.data?.stt_provider ?? null,
    ttsAvailable: query.data?.tts_configured === true,
    ttsProvider: query.data?.tts_provider ?? null,
    ocrAvailable: query.data?.ocr_available === true,
    imageAttachOk: query.data?.image_attach_ok === true,
    appleMedia: query.data?.apple_media ?? null,
  };
}
