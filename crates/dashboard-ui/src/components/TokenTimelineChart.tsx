import ReactECharts from "echarts-for-react";
import type { TokenTimelinePoint } from "@/api/types";
import { useSkin } from "@/hooks/useSkin";
import { useT } from "@/i18n/context";
import { chartPalette } from "@/lib/chartTheme";
import { fillTokenTimelineDays } from "@/lib/fillTimelineDays";
import { formatMoney } from "@/lib/money";

interface Props {
  points: TokenTimelinePoint[];
  days?: number;
  tall?: boolean;
}

export function TokenTimelineChart({ points, days = 7, tall }: Props) {
  const t = useT();
  const { skin } = useSkin();
  const palette = chartPalette();
  const filled = fillTokenTimelineDays(points, days);
  const hasActivity = filled.some((p) => p.llm_calls > 0 || p.total_tokens > 0);

  if (!hasActivity) {
    return <p className="text-sm text-secondary px-4 py-4 m-0">{t("charts.noTokenTimeline")}</p>;
  }

  const dates = filled.map((p) => p.date.slice(5));
  const tokens = filled.map((p) => p.total_tokens);
  const costs = filled.map((p) => Number(p.estimated_cost_cny.toFixed(4)));
  const showAllLabels = days <= 14;

  const option = {
    backgroundColor: "transparent",
    tooltip: {
      trigger: "axis",
      formatter: (params: { dataIndex: number }[]) => {
        const idx = params[0]?.dataIndex ?? 0;
        const p = filled[idx];
        if (!p) return "";
        return [
          p.date,
          `${t("home.tokenTotal")}: ${p.total_tokens.toLocaleString()}`,
          `${t("home.tokenCalls")}: ${p.llm_calls.toLocaleString()}`,
          `${t("home.tokenCost")}: ${formatMoney(p.estimated_cost_cny)}`,
        ].join("<br/>");
      },
    },
    legend: {
      data: [t("home.tokenTotal"), t("home.tokenCost")],
      textStyle: { color: palette.secondary, fontSize: 11 },
    },
    grid: { left: 52, right: 48, top: 40, bottom: showAllLabels ? 36 : 32 },
    xAxis: {
      type: "category",
      data: dates,
      boundaryGap: false,
      axisLabel: {
        color: palette.outline,
        fontSize: 10,
        interval: showAllLabels ? 0 : "auto",
        hideOverlap: !showAllLabels,
      },
      axisTick: { alignWithLabel: true },
    },
    yAxis: [
      {
        type: "value",
        name: "tokens",
        min: 0,
        splitNumber: 4,
        axisLabel: {
          color: palette.outline,
          fontSize: 10,
          formatter: (v: number) => formatAxisToken(v),
        },
        splitLine: { lineStyle: { opacity: 0.35 } },
      },
      {
        type: "value",
        name: "CNY",
        min: 0,
        splitNumber: 4,
        axisLabel: {
          color: palette.outline,
          fontSize: 10,
          formatter: (v: number) => (v >= 10 ? v.toFixed(0) : v.toFixed(2)),
        },
        splitLine: { show: false },
      },
    ],
    series: [
      {
        name: t("home.tokenTotal"),
        type: "line",
        smooth: 0.2,
        showSymbol: true,
        symbolSize: 6,
        data: tokens,
        itemStyle: { color: palette.primary },
        areaStyle: { color: palette.accentMuted, opacity: 0.35 },
      },
      {
        name: t("home.tokenCost"),
        type: "line",
        yAxisIndex: 1,
        smooth: 0.2,
        showSymbol: true,
        symbolSize: 5,
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

function formatAxisToken(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n % 1_000 === 0 ? 0 : 1)}k`;
  return String(n);
}
