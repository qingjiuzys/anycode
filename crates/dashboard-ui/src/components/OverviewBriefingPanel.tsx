import { useEffect } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { OverviewBriefing } from "@/api/types";
import { Icon } from "@/components/Icon";
import { SectionCard } from "@/components/ui/SectionCard";
import { TranscriptMarkdown } from "@/components/TranscriptMarkdown";
import { SecurityApprovalInbox } from "@/components/SecurityApprovalInbox";
import { useLocale, useT } from "@/i18n/context";

export function OverviewBriefingPanel({ active }: { active: boolean }) {
  const t = useT();
  const locale = useLocale();
  const lang = locale === "en" ? "en" : "zh";

  const gen = useMutation({
    mutationFn: () => api.overviewBriefing(7, lang),
  });

  useEffect(() => {
    if (active && !gen.data && !gen.isPending && !gen.isError) {
      gen.mutate();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- generate once when panel opens
  }, [active]);

  const briefing: OverviewBriefing | undefined = gen.data?.briefing;

  return (
    <div className="space-y-4">
      <SectionCard
        title={t("home.briefingTitle")}
        action={
          <button
            type="button"
            className="dw-btn-ghost text-xs"
            disabled={gen.isPending}
            onClick={() => gen.mutate()}
          >
            <Icon name="refresh" size={16} />
            {gen.isPending ? t("common.loading") : t("home.briefingRefresh")}
          </button>
        }
      >
        <p className="text-xs text-secondary m-0 mb-3 leading-relaxed">{t("home.briefingHint")}</p>
        {gen.isPending && !briefing ? (
          <p className="text-sm text-secondary m-0">{t("home.briefingGenerating")}</p>
        ) : gen.isError ? (
          <p className="text-sm text-error m-0">
            {gen.error instanceof Error ? gen.error.message : t("home.briefingError")}
          </p>
        ) : briefing ? (
          <>
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-secondary mb-3">
              <span>
                {t("home.briefingWindow").replace("{days}", String(briefing.window_days))}
              </span>
              <span>·</span>
              <span>
                {briefing.generation_mode === "llm"
                  ? t("home.briefingModeLlm")
                  : t("home.briefingModeTemplate")}
                {briefing.model ? ` · ${briefing.model}` : ""}
              </span>
            </div>
            {briefing.generation_mode === "template" && briefing.fallback_reason && (
              <p className="text-xs text-warn m-0 mb-3 leading-relaxed">
                {t("home.briefingFallbackHint")}
              </p>
            )}
            <div className="dw-briefing-md text-sm text-on-surface">
              <TranscriptMarkdown text={briefing.markdown} />
            </div>
          </>
        ) : (
          <p className="text-sm text-secondary m-0">{t("home.briefingEmpty")}</p>
        )}
      </SectionCard>
      <SecurityApprovalInbox />
    </div>
  );
}
