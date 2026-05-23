import ReactECharts from "echarts-for-react";
import type { ProjectSummary } from "@/api/types";
import { useT } from "@/i18n/context";

interface Props {
  projects: ProjectSummary[];
}

export function MetricsChart({ projects }: Props) {
  const t = useT();
  const names = projects.map((p) => p.name);
  const sessions = projects.map((p) => p.sessions_count);
  const trust = projects.map((p) => Math.round(p.trust_score * 100));

  const option = {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    legend: {
      data: [t("charts.sessionCount"), t("charts.trustScore")],
      textStyle: { color: "#505f76" },
    },
    grid: { left: 40, right: 20, top: 40, bottom: 30 },
    xAxis: {
      type: "category",
      data: names,
      axisLabel: { color: "#737686", fontSize: 11 },
    },
    yAxis: [
      {
        type: "value",
        name: t("charts.sessions"),
        axisLabel: { color: "#737686" },
      },
      {
        type: "value",
        name: t("charts.trustPct"),
        max: 100,
        axisLabel: { color: "#737686" },
      },
    ],
    series: [
      {
        name: t("charts.sessionCount"),
        type: "bar",
        data: sessions,
        itemStyle: { color: "#2563eb", borderRadius: [2, 2, 0, 0] },
      },
      {
        name: t("charts.trustScore"),
        type: "line",
        yAxisIndex: 1,
        data: trust,
        itemStyle: { color: "#16a34a" },
        lineStyle: { width: 2 },
      },
    ],
  };

  return (
    <div className="h-48">
      <ReactECharts option={option} style={{ height: "100%" }} />
    </div>
  );
}
