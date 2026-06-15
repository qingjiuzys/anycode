import ReactECharts from "echarts-for-react";
import type { TokenTimelinePoint } from "@/api/types";
import { useSkin } from "@/hooks/useSkin";
import { useT } from "@/i18n/context";
import { chartPalette } from "@/lib/chartTheme";

interface Props {
  points: TokenTimelinePoint[];
  tall?: boolean;
}

export function TokenTimelineChart({ points, tall }: Props) {
  const t = useT();
  const { skin } = useSkin();
  const palette = chartPalette();

  if (points.length === 0) {
    return <p className="text-sm text-secondary px-4 py-4 m-0">{t("charts.noTokenTimeline")}</p>;
  }

  const dates = points.map((p) => p.date.slice(5));
  const tokens = points.map((p) => p.total_tokens);
  const costs = points.map((p) => Number(p.estimated_cost_usd.toFixed(4)));
  const calls = points.map((p) => p.llm_calls);

  const option = {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    legend: {
      data: [t("home.tokenTotal"), t("home.tokenCost"), t("home.tokenCalls")],
      textStyle: { color: palette.secondary, fontSize: 11 },
    },
    grid: { left: 48, right: 48, top: 40, bottom: 32 },
    xAxis: {
      type: "category",
      data: dates,
      axisLabel: { color: palette.outline, fontSize: 10 },
    },
    yAxis: [
      {
        type: "value",
        name: "tokens",
        axisLabel: { color: palette.outline, fontSize: 10 },
      },
      {
        type: "value",
        name: "USD",
        axisLabel: { color: palette.outline, fontSize: 10 },
      },
    ],
    series: [
      {
        name: t("home.tokenTotal"),
        type: "line",
        smooth: true,
        data: tokens,
        itemStyle: { color: palette.primary },
        areaStyle: { color: palette.accentMuted },
      },
      {
        name: t("home.tokenCalls"),
        type: "bar",
        data: calls,
        itemStyle: { color: palette.success, borderRadius: [2, 2, 0, 0] },
      },
      {
        name: t("home.tokenCost"),
        type: "line",
        yAxisIndex: 1,
        data: costs,
        itemStyle: { color: palette.secondary },
        lineStyle: { width: 2 },
      },
    ],
  };

  return (
    <div className={`px-2 pb-3 ${tall ? "h-52 sm:h-56" : "h-44"}`}>
      <ReactECharts key={skin} option={option} style={{ height: "100%", width: "100%" }} />
    </div>
  );
}
