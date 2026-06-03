export interface AgentProfileRecord {
  id: string;
  scope: string;
  project_id: string | null;
  extends: string;
  description: string;
  tools_json: string;
  skills_json: string;
  routing_json: string;
  prompt_overlay: string;
  version: number;
  builtin: boolean;
  updated_at: string;
}

export interface AgentProfileUpsertBody {
  extends: string;
  description?: string;
  tools_json?: Record<string, unknown>;
  skills_json?: Record<string, unknown>;
  routing_json?: Record<string, unknown>;
  prompt_overlay?: string;
}

export interface AgentProfileEffective {
  id: string;
  extends: string;
  tools: string[];
  skills_json: Record<string, unknown>;
  routing_json: Record<string, unknown>;
}
