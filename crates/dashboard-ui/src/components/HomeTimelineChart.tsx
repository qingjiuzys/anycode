import ReactECharts from "echarts-for-react";
import type { GlobalTimelineMetrics } from "@/api/types";
import { useSkin } from "@/hooks/useSkin";
import { useT } from "@/i18n/context";
import { chartPalette } from "@/lib/chartTheme";

export function HomeTimelineChart({
  timeline,
  tall,
}: {
  timeline?: GlobalTimelineMetrics;
  tall?: boolean;
}) {
  const t = useT();
  const { skin } = useSkin();
  const palette = chartPalette();

  if (!timeline || timeline.points.length === 0) {
    return <p className="text-sm text-secondary px-4 py-6 m-0">{t("charts.noTimeline")}</p>;
  }

  const dates = timeline.points.map((p) => p.date.slice(5));
  const sessions = timeline.points.map((p) => p.sessions_count);
  const events = timeline.points.map((p) => p.events_count);
  const trend = timeline.trust_trend_pct;

  const option = {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    legend: {
      data: [t("charts.sessions"), t("charts.events")],
      textStyle: { color: palette.secondary },
    },
    grid: { left: 40, right: 12, top: 40, bottom: 32 },
    xAxis: {
      type: "category",
      data: dates,
      axisLabel: { color: palette.outline, fontSize: 10 },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: palette.outline, fontSize: 10 },
    },
    series: [
      {
        name: t("charts.sessions"),
        type: "line",
        smooth: true,
        data: sessions,
        itemStyle: { color: palette.primary },
        areaStyle: { color: palette.accentMuted },
      },
      {
        name: t("charts.events"),
        type: "bar",
        data: events,
        itemStyle: { color: palette.success, borderRadius: [2, 2, 0, 0] },
      },
    ],
  };

  return (
    <div>
      <div className="flex items-center justify-between px-4 pt-3 pb-1">
        <span className="text-xs text-secondary">{t("charts.timeline7d")}</span>
        <span
          className={`text-xs font-semibold tabular-nums ${trend >= 0 ? "text-success" : "text-error"}`}
        >
          {trend >= 0 ? "+" : ""}
          {trend.toFixed(1)}% {t("charts.throughputTrend")}
        </span>
      </div>
      <div className={`px-2 pb-3 ${tall ? "h-52 sm:h-56" : "h-44"}`}>
        <ReactECharts key={skin} option={option} style={{ height: "100%", width: "100%" }} />
      </div>
    </div>
  );
}
