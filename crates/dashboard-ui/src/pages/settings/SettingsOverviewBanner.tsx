import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { DataHealthPanel } from "@/components/DataHealthPanel";

export function SettingsOverviewBanner() {
  const dataHealth = useQuery({ queryKey: ["data-health"], queryFn: api.dataHealth });

  return <DataHealthPanel health={dataHealth.data?.health} compact />;
}
