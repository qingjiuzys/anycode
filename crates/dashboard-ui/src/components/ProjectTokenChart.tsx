import ReactECharts from "echarts-for-react";
import type { ProjectUsageRow } from "@/api/types";
import { useSkin } from "@/hooks/useSkin";
import { useT } from "@/i18n/context";
import { chartPalette } from "@/lib/chartTheme";

interface Props {
  rows: ProjectUsageRow[];
}

export function ProjectTokenChart({ rows }: Props) {
  const t = useT();
  const { skin } = useSkin();
  const palette = chartPalette();

  if (rows.length === 0) return null;

  const labels = rows.map((r) => r.project_name || r.project_id);
  const tokens = rows.map((r) => r.total_tokens);
  const costs = rows.map((r) => Number(r.estimated_cost_usd.toFixed(4)));

  const option = {
    backgroundColor: "transparent",
    tooltip: {
      trigger: "axis",
      formatter: (params: { name: string; dataIndex: number }[]) => {
        const idx = params[0]?.dataIndex ?? 0;
        const row = rows[idx];
        if (!row) return "";
        return [
          row.project_name,
          row.root_path,
          `${t("home.tokenTotal")}: ${row.total_tokens.toLocaleString()}`,
          `${t("home.tokenCost")}: $${row.estimated_cost_usd.toFixed(4)}`,
        ].join("<br/>");
      },
    },
    legend: {
      data: [t("home.tokenTotal"), t("home.tokenCost")],
      textStyle: { color: palette.secondary, fontSize: 11 },
    },
    grid: { left: 48, right: 48, top: 36, bottom: 28 },
    xAxis: {
      type: "category",
      data: labels,
      axisLabel: { color: palette.outline, fontSize: 10, rotate: labels.length > 4 ? 24 : 0 },
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
        type: "bar",
        data: tokens,
        itemStyle: { color: palette.primary, borderRadius: [2, 2, 0, 0] },
      },
      {
        name: t("home.tokenCost"),
        type: "line",
        yAxisIndex: 1,
        data: costs,
        itemStyle: { color: palette.success },
        lineStyle: { width: 2 },
      },
    ],
  };

  return (
    <div className="h-44 mt-3">
      <ReactECharts key={skin} option={option} style={{ height: "100%" }} />
    </div>
  );
}
