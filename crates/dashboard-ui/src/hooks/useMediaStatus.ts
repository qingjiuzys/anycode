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
    appleMedia: query.data?.apple_media ?? null,
  };
}
