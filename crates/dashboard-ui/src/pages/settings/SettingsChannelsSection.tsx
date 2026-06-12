import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { DiscordChannelPanel } from "@/components/channels/DiscordChannelPanel";
import { TelegramChannelPanel } from "@/components/channels/TelegramChannelPanel";
import { useT } from "@/i18n/context";

type ChannelCard = "telegram" | "discord" | "wechat";

export function SettingsChannelsSection() {
  const t = useT();
  const [active, setActive] = useState<ChannelCard>("telegram");

  const settings = useQuery({
    queryKey: ["channels-settings"],
    queryFn: () => api.channelsSettings(),
  });

  const ch = settings.data?.channels;

  return (
    <section className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold m-0">{t("settings.channels.title")}</h2>
        <p className="text-secondary text-sm mt-1 mb-0">{t("settings.channels.subtitle")}</p>
      </div>

      <div className="flex flex-wrap gap-2">
        {(["telegram", "discord", "wechat"] as const).map((id) => {
          const configured =
            id === "telegram"
              ? ch?.telegram.configured
              : id === "discord"
                ? ch?.discord.configured
                : ch?.wechat;
          return (
            <button
              key={id}
              type="button"
              className={`dw-btn dw-btn-secondary text-sm${active === id ? " dw-btn-primary" : ""}`}
              onClick={() => setActive(id)}
            >
              {t(`settings.channels.card.${id}`)}
              {configured && (
                <span className="ml-1.5 text-xs opacity-80">({t("settings.channels.configured")})</span>
              )}
            </button>
          );
        })}
      </div>

      {active === "telegram" && (
        <div className="dw-card p-4">
          {ch?.telegram.configured && ch.telegram.chat_id && (
            <p className="text-secondary text-sm mb-3">
              {t("settings.channels.savedChatId").replace("{id}", ch.telegram.chat_id)}
            </p>
          )}
          <TelegramChannelPanel
            initialChatId={ch?.telegram.chat_id}
            startCommand={ch?.telegram_start_command ?? "anycode channel telegram"}
          />
        </div>
      )}

      {active === "discord" && (
        <div className="dw-card p-4">
          {ch?.discord.configured && ch.discord.channel_id && (
            <p className="text-secondary text-sm mb-3">
              {t("settings.channels.savedChannelId").replace("{id}", ch.discord.channel_id)}
            </p>
          )}
          <DiscordChannelPanel
            initialChannelId={ch?.discord.channel_id}
            startCommand={ch?.discord_start_command ?? "anycode channel discord"}
          />
        </div>
      )}

      {active === "wechat" && (
        <div className="dw-card p-4">
          <p className="text-secondary text-sm mb-3">
            {ch?.wechat ? t("settings.channels.wechatConfigured") : t("settings.channels.wechatHint")}
          </p>
          <Link
            to="/setup"
            search={{ review: "1", step: "channels", tab: "wechat" }}
            className="dw-btn dw-btn-secondary text-sm"
          >
            {t("settings.channels.wechatSetup")}
          </Link>
        </div>
      )}
    </section>
  );
}
