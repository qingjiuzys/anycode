import ReactECharts from "echarts-for-react";
import type { ProjectSummary } from "@/api/types";
import { useSkin } from "@/hooks/useSkin";
import { useT } from "@/i18n/context";
import { chartPalette } from "@/lib/chartTheme";

interface Props {
  projects: ProjectSummary[];
}

export function MetricsChart({ projects, tall }: Props & { tall?: boolean }) {
  const t = useT();
  const { skin } = useSkin();
  const palette = chartPalette();
  const names = projects.map((p) => p.name);
  const sessions = projects.map((p) => p.sessions_count);
  const trust = projects.map((p) =>
    p.trust_score == null ? null : Math.round(p.trust_score * 100),
  );

  const option = {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    legend: {
      data: [t("charts.sessionCount"), t("charts.trustScore")],
      textStyle: { color: palette.secondary },
    },
    grid: { left: 44, right: 16, top: 44, bottom: tall ? 56 : 36 },
    xAxis: {
      type: "category",
      data: names,
      axisLabel: {
        color: palette.outline,
        fontSize: 10,
        rotate: tall && names.length > 4 ? 24 : 0,
      },
    },
    yAxis: [
      {
        type: "value",
        name: t("charts.sessions"),
        axisLabel: { color: palette.outline },
      },
      {
        type: "value",
        name: t("charts.trustPct"),
        max: 100,
        axisLabel: { color: palette.outline },
      },
    ],
    series: [
      {
        name: t("charts.sessionCount"),
        type: "bar",
        data: sessions,
        itemStyle: { color: palette.primary, borderRadius: [2, 2, 0, 0] },
      },
      {
        name: t("charts.trustScore"),
        type: "line",
        yAxisIndex: 1,
        data: trust,
        itemStyle: { color: palette.success },
        lineStyle: { width: 2 },
      },
    ],
  };

  return (
    <div className={tall ? "h-52 sm:h-56" : "h-48"}>
      <ReactECharts
        key={skin}
        option={option}
        style={{ height: "100%", width: "100%" }}
      />
    </div>
  );
}
