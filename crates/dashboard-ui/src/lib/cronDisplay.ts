type Locale = "en" | "zh";

const DOW_LABELS: Record<Locale, Record<string, string>> = {
  en: {
    "0": "Sunday",
    "1": "Monday",
    "2": "Tuesday",
    "3": "Wednesday",
    "4": "Thursday",
    "5": "Friday",
    "6": "Saturday",
    "1-5": "Weekdays",
    "*": "Daily",
  },
  zh: {
    "0": "星期日",
    "1": "星期一",
    "2": "星期二",
    "3": "星期三",
    "4": "星期四",
    "5": "星期五",
    "6": "星期六",
    "1-5": "工作日",
    "*": "每天",
  },
};

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

function formatClock(hour: number, minute: number): string {
  return `${pad2(hour)}:${pad2(minute)}`;
}

export function formatCronScheduleLabel(schedule: string, locale: Locale): string {
  const parts = schedule.trim().split(/\s+/);
  if (parts.length < 6) return schedule;
  const minute = Number(parts[1]);
  const hour = Number(parts[2]);
  const dow = parts[5];
  if (!Number.isFinite(minute) || !Number.isFinite(hour)) return schedule;

  const clock = formatClock(hour, minute);
  const dowLabel = DOW_LABELS[locale][dow] ?? dow;
  if (dow === "*" || dow === "?") {
    return locale === "zh" ? `每天 ${clock}` : `Daily ${clock}`;
  }
  if (dow === "1-5") {
    return locale === "zh" ? `工作日 ${clock}` : `Weekdays ${clock}`;
  }
  return locale === "zh" ? `${dowLabel} ${clock}` : `${dowLabel} ${clock}`;
}

export function formatNextRunRelative(iso: string | null | undefined, locale: Locale): string {
  if (!iso) return locale === "zh" ? "暂无计划" : "Not scheduled";
  const target = new Date(iso);
  if (Number.isNaN(target.getTime())) return locale === "zh" ? "暂无计划" : "Not scheduled";

  const diffMs = target.getTime() - Date.now();
  const abs = Math.abs(diffMs);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (diffMs <= 0) {
    return locale === "zh" ? "即将运行" : "Due now";
  }
  if (abs < hour) {
    const mins = Math.max(1, Math.round(abs / minute));
    return locale === "zh" ? `${mins} 分钟后` : `in ${mins} min`;
  }
  if (abs < day) {
    const hours = Math.max(1, Math.round(abs / hour));
    return locale === "zh" ? `${hours} 小时后` : `in ${hours} h`;
  }
  const days = Math.max(1, Math.round(abs / day));
  return locale === "zh" ? `${days} 天后` : `in ${days} d`;
}

export function formatNextRunLine(
  schedule: string,
  nextRunAt: string | null | undefined,
  locale: Locale,
): string {
  const scheduleLabel = formatCronScheduleLabel(schedule, locale);
  const nextLabel = formatNextRunRelative(nextRunAt, locale);
  return locale === "zh"
    ? `${scheduleLabel} · 下次运行 ${nextLabel}`
    : `${scheduleLabel} · Next run ${nextLabel}`;
}
