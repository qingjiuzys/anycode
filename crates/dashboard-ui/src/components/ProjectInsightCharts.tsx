import ReactECharts from "echarts-for-react";
import type { ProjectStats } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useT } from "@/i18n/context";

interface Props {
  stats: ProjectStats;
}

export function ProjectInsightCharts({ stats }: Props) {
  const t = useT();
  const eventTypeOption = pieOption(
    t("charts.eventTypes"),
    stats.event_types.map((x) => x.label),
    stats.event_types.map((x) => x.count),
  );
  const gateOption = pieOption(
    t("charts.gateStatus"),
    stats.gate_statuses.map((x) => x.label),
    stats.gate_statuses.map((x) => x.count),
  );
  const sessionOption = barOption(
    t("charts.sessionStatus"),
    stats.session_statuses.map((x) => x.label),
    stats.session_statuses.map((x) => x.count),
  );

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <SectionCard title={t("charts.eventDistribution")}>
        {stats.event_types.length > 0 ? (
          <div className="h-[220px]">
            <ReactECharts option={eventTypeOption} style={{ height: "100%" }} />
          </div>
        ) : (
          <p className="text-sm text-secondary m-0">{t("events.empty")}</p>
        )}
      </SectionCard>
      <SectionCard title={t("charts.gateAndSessions")}>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div className="h-[200px]">
            {stats.gate_statuses.length > 0 ? (
              <ReactECharts option={gateOption} style={{ height: "100%" }} />
            ) : (
              <p className="text-sm text-secondary m-0">{t("charts.noGates")}</p>
            )}
          </div>
          <div className="h-[200px]">
            {stats.session_statuses.length > 0 ? (
              <ReactECharts option={sessionOption} style={{ height: "100%" }} />
            ) : (
              <p className="text-sm text-secondary m-0">{t("charts.noSessions")}</p>
            )}
          </div>
        </div>
      </SectionCard>
    </div>
  );
}

function pieOption(title: string, labels: string[], values: number[]) {
  return {
    backgroundColor: "transparent",
    tooltip: { trigger: "item" },
    legend: {
      orient: "vertical",
      right: 0,
      top: "center",
      textStyle: { color: "#737686", fontSize: 11 },
    },
    series: [
      {
        name: title,
        type: "pie",
        radius: ["40%", "65%"],
        center: ["35%", "50%"],
        data: labels.map((name, i) => ({ name, value: values[i] })),
        label: { show: false },
      },
    ],
  };
}

function barOption(title: string, labels: string[], values: number[]) {
  return {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    grid: { left: 40, right: 10, top: 20, bottom: 30 },
    xAxis: {
      type: "category",
      data: labels,
      axisLabel: { color: "#737686", fontSize: 10 },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: "#737686" },
    },
    series: [
      {
        name: title,
        type: "bar",
        data: values,
        itemStyle: { color: "#2563eb" },
      },
    ],
  };
}
