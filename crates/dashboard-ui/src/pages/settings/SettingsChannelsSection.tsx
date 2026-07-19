import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import { DiscordChannelPanel } from "@/components/channels/DiscordChannelPanel";
import { TelegramChannelPanel } from "@/components/channels/TelegramChannelPanel";
import { WeChatChannelPanel } from "@/components/channels/WeChatChannelPanel";
import { SectionCard } from "@/components/ui/SectionCard";
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
        <h2 className="text-base font-semibold m-0 text-on-surface">{t("settings.channels.title")}</h2>
        <p className="text-secondary text-sm mt-1 mb-0">{t("settings.channels.subtitle")}</p>
      </div>

      <div className="flex flex-wrap gap-2 sticky top-0 z-[1] py-1 bg-background">
        {(["telegram", "discord", "wechat"] as const).map((id) => {
          const configured =
            id === "telegram"
              ? ch?.telegram.configured
              : id === "discord"
                ? ch?.discord.configured
                : ch?.wechat;
          const selected = active === id;
          return (
            <button
              key={id}
              type="button"
              className={selected ? "dw-btn-primary text-sm" : "dw-btn-secondary text-sm"}
              aria-pressed={selected}
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
        <SectionCard>
          {ch?.telegram.configured && ch.telegram.chat_id && (
            <p className="text-secondary text-sm mb-3 m-0">
              {t("settings.channels.savedChatId").replace("{id}", ch.telegram.chat_id)}
            </p>
          )}
          <TelegramChannelPanel
            initialChatId={ch?.telegram.chat_id}
            startCommand={ch?.telegram_start_command ?? "anycode-daemon telegram-bridge"}
          />
        </SectionCard>
      )}

      {active === "discord" && (
        <SectionCard>
          {ch?.discord.configured && ch.discord.channel_id && (
            <p className="text-secondary text-sm mb-3 m-0">
              {t("settings.channels.savedChannelId").replace("{id}", ch.discord.channel_id)}
            </p>
          )}
          <DiscordChannelPanel
            initialChannelId={ch?.discord.channel_id}
            startCommand={ch?.discord_start_command ?? "anycode-daemon discord-bridge"}
          />
        </SectionCard>
      )}

      {active === "wechat" && (
        <SectionCard>
          <WeChatChannelPanel
            configured={Boolean(ch?.wechat)}
            platform={ch?.platform}
            startCommand="anycode-daemon wechat-bridge"
          />
        </SectionCard>
      )}
    </section>
  );
}
