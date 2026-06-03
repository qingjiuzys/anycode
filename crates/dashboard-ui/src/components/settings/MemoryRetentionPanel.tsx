import { useMutation, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api } from "@/api/client";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

export function MemoryRetentionPanel() {
  const t = useT();
  const [days, setDays] = useState(90);
  const [msg, setMsg] = useState("");

  const preview = useQuery({
    queryKey: ["memory-retention-preview", days],
    queryFn: () => api.memoryRetentionPreview(days),
    staleTime: 30_000,
  });

  const apply = useMutation({
    mutationFn: () => api.memoryRetentionApply(days, true),
    onSuccess: (data) => {
      const s = data.summary;
      setMsg(
        t("settings.memoryApplyOk")
          .replace("{deleted}", String(s?.would_delete ?? 0))
          .replace("{keep}", String(s?.keep ?? 0)),
      );
      void preview.refetch();
    },
    onError: (e: Error) => setMsg(e.message),
  });

  const summary = preview.data?.summary;

  return (
    <SectionCard title={t("settings.memoryRetention")}>
      <p className="text-sm text-secondary m-0 mb-3">{t("settings.memoryRetentionHint")}</p>
      <label className="block text-xs text-secondary mb-1">{t("settings.memoryOlderThan")}</label>
      <input
        type="number"
        min={1}
        max={3650}
        className="dw-input w-32 mb-3"
        value={days}
        onChange={(e) => setDays(Number(e.target.value) || 90)}
      />
      {summary && (
        <p className="text-sm m-0 mb-2">
          {t("settings.memorySummary")
            .replace("{delete}", String(summary.would_delete ?? 0))
            .replace("{keep}", String(summary.keep ?? 0))
            .replace("{protected}", String(summary.protected ?? 0))}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className="dw-btn-secondary text-sm"
          disabled={preview.isFetching}
          onClick={() => void preview.refetch()}
        >
          {preview.isFetching ? "…" : t("settings.memoryPreview")}
        </button>
        <button
          type="button"
          className="dw-btn-primary text-sm"
          disabled={apply.isPending}
          onClick={() => {
            if (
              window.confirm(
                t("settings.memoryApplyConfirm").replace("{days}", String(days)),
              )
            ) {
              apply.mutate();
            }
          }}
        >
          {apply.isPending ? "…" : t("settings.memoryApply")}
        </button>
      </div>
      {msg && <p className="text-sm text-secondary mt-2 m-0">{msg}</p>}
      <p className="text-xs text-secondary mt-3 m-0">
        CLI: <code className="font-code">anycode memory prune --dry-run</code>
      </p>
    </SectionCard>
  );
}
