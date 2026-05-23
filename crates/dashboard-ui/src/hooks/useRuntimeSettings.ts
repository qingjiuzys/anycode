import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";

export function useRuntimeSettings() {
  return useQuery({ queryKey: ["runtime-settings"], queryFn: api.runtimeSettings });
}
