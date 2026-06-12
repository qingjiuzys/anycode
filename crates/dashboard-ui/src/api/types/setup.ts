export type SetupStepId =
  | "workspace"
  | "llm"
  | "llm_test"
  | "memory"
  | "skills"
  | "channels"
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
};

export type WechatQrPayload = {
  qrcode_id: string;
  content: string;
  terminal_render?: string | null;
};

export type WechatQrPollResult = {
  status: "wait" | "scanned" | "confirmed" | "expired" | "error";
  message?: string | null;
  account_saved: boolean;
};

export type TelegramBotInfo = {
  id: number;
  username: string;
  first_name?: string | null;
};

export type TelegramChatOption = {
  chat_id: string;
  title?: string | null;
  username?: string | null;
  chat_type?: string | null;
};

export type DiscordBotInfo = {
  id: string;
  username: string;
  global_name?: string | null;
};

export type ChannelsSettingsView = {
  telegram: { configured: boolean; chat_id?: string | null; path?: string | null };
  discord: { configured: boolean; channel_id?: string | null; path?: string | null };
  wechat: boolean;
  platform: string;
  telegram_start_command: string;
  discord_start_command: string;
};
