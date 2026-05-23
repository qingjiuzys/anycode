import type { ProjectReadinessItem } from "./project";

export interface DeliveryReadiness {
  status: string;
  blocked_sessions: number;
  failed_required_gates: number;
  unverified_artifacts: number;
  stale_running_sessions: number;
  running_sessions: number;
  projects: ProjectReadinessItem[];
  generated_at: string;
}

export interface TimelineMetricPoint {
  date: string;
  sessions_count: number;
  events_count: number;
  gates_failed: number;
}

export interface GlobalTimelineMetrics {
  days: number;
  points: TimelineMetricPoint[];
  trust_trend_pct: number;
  generated_at: string;
}

export interface TokenUsageStats {
  days: number;
  llm_calls: number;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  estimated_cost_usd: number;
  generated_at: string;
}

export interface ModelUsageRow {
  model: string;
  provider: string;
  llm_calls: number;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  estimated_cost_usd: number;
}

export interface TokenUsageDetail {
  usage: TokenUsageStats;
  by_model: ModelUsageRow[];
}

export interface SavedHoursKpi {
  days: number;
  sessions_completed: number;
  automation_hours: number;
  baseline_hours_per_session: number;
  estimated_manual_hours: number;
  estimated_saved_hours: number;
  hourly_rate_usd: number;
  estimated_value_usd: number;
  generated_at: string;
}
