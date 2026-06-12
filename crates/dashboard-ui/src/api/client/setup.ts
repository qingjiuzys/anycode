import type {
  ChannelsSettingsView,
  DiscordBotInfo,
  QuickAuthPreset,
  SetupStatus,
  TelegramBotInfo,
  TelegramChatOption,
  WechatQrPayload,
  WechatQrPollResult,
} from "../types/setup";
import { get, patch, post } from "../http";

export const setupClient = {
  setupStatus: () => get<{ setup: SetupStatus }>("/api/setup/status"),
  setupQuickAuth: () =>
    get<{ presets: QuickAuthPreset[] }>("/api/setup/quick-auth"),
  setupEnsureWorkspace: () =>
    post<{ ok: boolean; error?: string }>("/api/setup/workspace/ensure", {}),
  setupMemory: (body: {
    preset: string;
    embedding_base_url?: string;
    embedding_model?: string;
  }) => patch<{ ok: boolean; config_path?: string; error?: string }>("/api/setup/memory", body),
  channelsSettings: () => get<{ channels: ChannelsSettingsView }>("/api/settings/channels"),
  setupTelegramVerify: (bot_token: string) =>
    post<{ ok: boolean; bot?: TelegramBotInfo; error?: string }>(
      "/api/setup/channels/telegram/verify",
      { bot_token },
    ),
  setupTelegramChats: (bot_token: string) =>
    post<{ ok: boolean; chats?: TelegramChatOption[]; error?: string }>(
      "/api/setup/channels/telegram/chats",
      { bot_token },
    ),
  setupTelegram: (body: { bot_token: string; chat_id?: string }) =>
    post<{ ok: boolean; path?: string; error?: string }>(
      "/api/setup/channels/telegram",
      body,
    ),
  setupDiscordVerify: (bot_token: string) =>
    post<{ ok: boolean; bot?: DiscordBotInfo; invite_url?: string; error?: string }>(
      "/api/setup/channels/discord/verify",
      { bot_token },
    ),
  setupDiscordTest: (body: { bot_token: string; channel_id: string }) =>
    post<{ ok: boolean; result?: { message_id: string; channel_id: string }; error?: string }>(
      "/api/setup/channels/discord/test",
      body,
    ),
  setupDiscord: (body: { bot_token: string; channel_id: string }) =>
    post<{ ok: boolean; path?: string; error?: string }>(
      "/api/setup/channels/discord",
      body,
    ),
  setupWechatQr: () =>
    get<{ ok: boolean; qr?: WechatQrPayload; error?: string }>(
      "/api/setup/channels/wechat/qr",
    ),
  setupWechatStatus: (qrcodeId: string) =>
    get<{ ok: boolean; result?: WechatQrPollResult; error?: string }>(
      `/api/setup/channels/wechat/status?qrcode_id=${encodeURIComponent(qrcodeId)}`,
    ),
  setupComplete: (body?: { scan_projects?: boolean }) =>
    post<{ ok: boolean; setup_completed_at?: string; error?: string }>(
      "/api/setup/complete",
      body ?? { scan_projects: true },
    ),
};
