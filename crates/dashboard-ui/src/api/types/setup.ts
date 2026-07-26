export type SetupStepId =
  | "workspace"
  | "llm"
  | "llm_test"
  | "memory"
  | "skills"
  | "projects"
  | "done";

export type SetupStepStatus = {
  id: SetupStepId;
  complete: boolean;
  optional: boolean;
};

export type SetupStatus = {
  ready: boolean;
  config_path: string;
  platform: string;
  setup_completed_at?: string | null;
  steps: SetupStepStatus[];
};

export type QuickAuthPreset = {
  id: string;
  label: string;
  provider: string;
  plan: string;
  default_model: string;
  base_url: string;
  key_envs: string[];
  device_auth?: boolean;
};

