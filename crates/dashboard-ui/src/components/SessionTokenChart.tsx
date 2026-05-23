import ReactECharts from "echarts-for-react";
import type { ModelUsageRow } from "@/api/types";
import { useT } from "@/i18n/context";

interface Props {
  rows: ModelUsageRow[];
}

export function SessionTokenChart({ rows }: Props) {
  const t = useT();
  if (rows.length === 0) return null;

  const labels = rows.map((r) => r.model || r.provider || "unknown");
  const tokens = rows.map((r) => r.total_tokens);
  const costs = rows.map((r) => Number(r.estimated_cost_usd.toFixed(4)));

  const option = {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    legend: {
      data: [t("home.tokenTotal"), t("home.tokenCost")],
      textStyle: { color: "#505f76", fontSize: 11 },
    },
    grid: { left: 48, right: 48, top: 36, bottom: 28 },
    xAxis: {
      type: "category",
      data: labels,
      axisLabel: { color: "#737686", fontSize: 10, rotate: labels.length > 4 ? 24 : 0 },
    },
    yAxis: [
      {
        type: "value",
        name: "tokens",
        axisLabel: { color: "#737686", fontSize: 10 },
      },
      {
        type: "value",
        name: "USD",
        axisLabel: { color: "#737686", fontSize: 10 },
      },
    ],
    series: [
      {
        name: t("home.tokenTotal"),
        type: "bar",
        data: tokens,
        itemStyle: { color: "#2563eb", borderRadius: [2, 2, 0, 0] },
      },
      {
        name: t("home.tokenCost"),
        type: "line",
        yAxisIndex: 1,
        data: costs,
        itemStyle: { color: "#16a34a" },
        lineStyle: { width: 2 },
      },
    ],
  };

  return (
    <div className="h-44 mt-3">
      <ReactECharts option={option} style={{ height: "100%" }} />
    </div>
  );
}
