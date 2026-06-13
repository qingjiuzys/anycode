import ReactECharts from "echarts-for-react";
import type { ProjectStats } from "@/api/types";
import { SectionCard } from "@/components/ui/SectionCard";
import { useSkin } from "@/hooks/useSkin";
import { useT } from "@/i18n/context";
import { chartPalette } from "@/lib/chartTheme";

interface Props {
  stats: ProjectStats;
}

export function ProjectInsightCharts({ stats }: Props) {
  const t = useT();
  const { skin } = useSkin();
  const palette = chartPalette();

  const eventTypeOption = pieOption(
    t("charts.eventTypes"),
    stats.event_types.map((x) => x.label),
    stats.event_types.map((x) => x.count),
    palette,
  );
  const gateOption = pieOption(
    t("charts.gateStatus"),
    stats.gate_statuses.map((x) => x.label),
    stats.gate_statuses.map((x) => x.count),
    palette,
  );
  const sessionOption = barOption(
    t("charts.sessionStatus"),
    stats.session_statuses.map((x) => x.label),
    stats.session_statuses.map((x) => x.count),
    palette,
  );

  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <SectionCard title={t("charts.eventDistribution")} className="dw-project-chart-card">
        {stats.event_types.length > 0 ? (
          <div className="h-[240px]">
            <ReactECharts
              key={`events-${skin}`}
              option={eventTypeOption}
              style={{ height: "100%", width: "100%" }}
            />
          </div>
        ) : (
          <p className="text-sm text-secondary m-0 py-8 text-center">{t("events.empty")}</p>
        )}
      </SectionCard>
      <SectionCard title={t("charts.gateAndSessions")} className="dw-project-chart-card">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 min-h-[240px]">
          <div className="flex flex-col min-h-[200px]">
            {stats.gate_statuses.length > 0 ? (
              <ReactECharts
                key={`gates-${skin}`}
                option={gateOption}
                style={{ height: "200px", width: "100%" }}
              />
            ) : (
              <div className="flex-1 flex items-center justify-center">
                <p className="text-sm text-secondary m-0">{t("charts.noGates")}</p>
              </div>
            )}
          </div>
          <div className="flex flex-col min-h-[200px]">
            {stats.session_statuses.length > 0 ? (
              <ReactECharts
                key={`sessions-${skin}`}
                option={sessionOption}
                style={{ height: "200px", width: "100%" }}
              />
            ) : (
              <div className="flex-1 flex items-center justify-center">
                <p className="text-sm text-secondary m-0">{t("charts.noSessions")}</p>
              </div>
            )}
          </div>
        </div>
      </SectionCard>
    </div>
  );
}

function pieOption(
  title: string,
  labels: string[],
  values: number[],
  palette: ReturnType<typeof chartPalette>,
) {
  return {
    backgroundColor: "transparent",
    tooltip: { trigger: "item" },
    legend: {
      orient: "vertical",
      right: 0,
      top: "center",
      textStyle: { color: palette.outline, fontSize: 11 },
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

function barOption(
  title: string,
  labels: string[],
  values: number[],
  palette: ReturnType<typeof chartPalette>,
) {
  return {
    backgroundColor: "transparent",
    tooltip: { trigger: "axis" },
    grid: { left: 40, right: 10, top: 20, bottom: 36 },
    xAxis: {
      type: "category",
      data: labels,
      axisLabel: { color: palette.outline, fontSize: 10 },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: palette.outline },
    },
    series: [
      {
        name: title,
        type: "bar",
        data: values,
        itemStyle: { color: palette.primary, borderRadius: [3, 3, 0, 0] },
      },
    ],
  };
}
