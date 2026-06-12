import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { DiscordBotInfo } from "@/api/types/setup";
import { api } from "@/api/client";
import {
  ChannelGuideSteps,
  DISCORD_GUIDE_STEPS,
} from "@/components/channels/ChannelGuideSteps";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useT } from "@/i18n/context";

export function DiscordChannelPanel({
  initialChannelId,
  startCommand = "anycode channel discord",
  compact,
}: {
  initialChannelId?: string | null;
  startCommand?: string;
  compact?: boolean;
}) {
  const t = useT();
  const qc = useQueryClient();
  const [token, setToken] = useState("");
  const [channelId, setChannelId] = useState(initialChannelId ?? "");
  const [verifiedBot, setVerifiedBot] = useState<DiscordBotInfo | null>(null);
  const [inviteUrl, setInviteUrl] = useState<string | null>(null);
  const [testedOk, setTestedOk] = useState(false);

  const verify = useMutation({
    mutationFn: () => api.setupDiscordVerify(token.trim()),
    onSuccess: (data) => {
      if (data.ok && data.bot) {
        setVerifiedBot(data.bot);
        setInviteUrl(data.invite_url ?? null);
        setTestedOk(false);
      }
    },
  });

  const test = useMutation({
    mutationFn: () =>
      api.setupDiscordTest({ bot_token: token.trim(), channel_id: channelId.trim() }),
    onSuccess: (data) => {
      setTestedOk(Boolean(data.ok));
    },
    onError: () => setTestedOk(false),
  });

  const save = useMutation({
    mutationFn: () =>
      api.setupDiscord({ bot_token: token.trim(), channel_id: channelId.trim() }),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["channels-settings"] });
    },
  });

  const copyCommand = async () => {
    try {
      await navigator.clipboard.writeText(startCommand);
    } catch {
      /* ignore */
    }
  };

  const canSave = token.trim() && channelId.trim() && verifiedBot && testedOk;

  return (
    <div className="channel-panel">
      {!compact && <ChannelGuideSteps steps={DISCORD_GUIDE_STEPS} prefix="discord" />}

      <label className="flex flex-col gap-1 text-sm mb-3">
        <span className="text-secondary font-medium">{t("channels.discord.tokenLabel")}</span>
        <input
          className="dw-input font-code"
          type="password"
          value={token}
          onChange={(e) => {
            setToken(e.target.value);
            setVerifiedBot(null);
            setInviteUrl(null);
            setTestedOk(false);
          }}
          placeholder={t("channels.discord.tokenPlaceholder")}
          autoComplete="off"
        />
      </label>

      <button
        type="button"
        className="dw-btn dw-btn-secondary text-sm mb-3"
        disabled={verify.isPending || !token.trim()}
        onClick={() => verify.mutate()}
      >
        {verify.isPending ? t("channels.discord.verifying") : t("channels.discord.verify")}
      </button>

      {verify.isError && (
        <div className="dw-alert-error mb-3 text-sm">{(verify.error as Error).message}</div>
      )}

      {verifiedBot && (
        <div className="dw-alert-success mb-3 text-sm">
          {t("channels.discord.verified").replace("{username}", verifiedBot.username)}
        </div>
      )}

      {inviteUrl && (
        <p className="text-sm mb-3">
          {t("channels.discord.inviteHint")}{" "}
          <ExternalNavLink href={inviteUrl} className="dw-link">
            {t("channels.discord.inviteLink")}
          </ExternalNavLink>
        </p>
      )}

      {verifiedBot && (
        <>
          <label className="flex flex-col gap-1 text-sm mb-3">
            <span className="text-secondary font-medium">{t("channels.discord.channelLabel")}</span>
            <input
              className="dw-input font-code"
              value={channelId}
              onChange={(e) => {
                setChannelId(e.target.value);
                setTestedOk(false);
              }}
              placeholder={t("setup.channels.dcChannel")}
            />
          </label>

          <button
            type="button"
            className="dw-btn dw-btn-secondary text-sm mb-3"
            disabled={test.isPending || !channelId.trim()}
            onClick={() => test.mutate()}
          >
            {test.isPending ? t("channels.discord.testing") : t("channels.discord.testMessage")}
          </button>

          {test.isSuccess && testedOk && (
            <div className="dw-alert-success mb-3 text-sm">{t("channels.discord.testOk")}</div>
          )}
          {test.isError && (
            <div className="dw-alert-error mb-3 text-sm">{(test.error as Error).message}</div>
          )}
          {!testedOk && channelId.trim() && (
            <p className="text-secondary text-xs mb-3">{t("channels.discord.testRequired")}</p>
          )}
        </>
      )}

      {save.isSuccess && (
        <div className="dw-alert-success mb-3 text-sm">
          {t("setup.channels.saved")}
          {save.data?.path && (
            <span className="block font-code text-xs mt-1">{save.data.path}</span>
          )}
        </div>
      )}
      {save.isError && (
        <div className="dw-alert-error mb-3 text-sm">{(save.error as Error).message}</div>
      )}

      <button
        type="button"
        className="dw-btn dw-btn-primary text-sm mb-2"
        disabled={save.isPending || !canSave}
        onClick={() => save.mutate()}
      >
        {save.isPending ? t("common.saving") : t("common.save")}
      </button>

      {save.isSuccess && (
        <div className="text-sm text-secondary">
          <p className="m-0 mb-1">{t("channels.startBridgeHint")}</p>
          <code className="font-code text-xs">{startCommand}</code>{" "}
          <button type="button" className="dw-link text-sm" onClick={() => void copyCommand()}>
            {t("common.copy")}
          </button>
        </div>
      )}

      <p className="text-xs text-secondary mt-3 m-0">
        <ExternalNavLink href="https://docs.anycode.dev/guide/discord" className="dw-link">
          {t("channels.docsLink")}
        </ExternalNavLink>
      </p>
    </div>
  );
}
