import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { TelegramBotInfo, TelegramChatOption } from "@/api/types/setup";
import { api } from "@/api/client";
import {
  ChannelGuideSteps,
  TELEGRAM_GUIDE_STEPS,
} from "@/components/channels/ChannelGuideSteps";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useT } from "@/i18n/context";

function formatChatLabel(chat: TelegramChatOption) {
  const name = chat.title || chat.username || chat.chat_id;
  const kind = chat.chat_type ?? "chat";
  return `${name} (${kind}, id: ${chat.chat_id})`;
}

export function TelegramChannelPanel({
  initialChatId,
  startCommand = "anycode channel telegram",
  compact,
}: {
  initialChatId?: string | null;
  startCommand?: string;
  compact?: boolean;
}) {
  const t = useT();
  const qc = useQueryClient();
  const [token, setToken] = useState("");
  const [chatId, setChatId] = useState(initialChatId ?? "");
  const [verifiedBot, setVerifiedBot] = useState<TelegramBotInfo | null>(null);
  const [chats, setChats] = useState<TelegramChatOption[]>([]);

  const verify = useMutation({
    mutationFn: () => api.setupTelegramVerify(token.trim()),
    onSuccess: (data) => {
      if (data.ok && data.bot) {
        setVerifiedBot(data.bot);
        setChats([]);
      }
    },
  });

  const fetchChats = useMutation({
    mutationFn: () => api.setupTelegramChats(token.trim()),
    onSuccess: (data) => {
      if (data.ok && data.chats) {
        setChats(data.chats);
        if (data.chats.length === 1 && !chatId) {
          setChatId(data.chats[0].chat_id);
        }
      }
    },
  });

  const save = useMutation({
    mutationFn: () =>
      api.setupTelegram({
        bot_token: token.trim(),
        chat_id: chatId.trim() || undefined,
      }),
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

  return (
    <div className="channel-panel">
      {!compact && <ChannelGuideSteps steps={TELEGRAM_GUIDE_STEPS} prefix="telegram" />}

      <label className="flex flex-col gap-1 text-sm mb-3">
        <span className="text-secondary font-medium">{t("channels.telegram.tokenLabel")}</span>
        <input
          className="dw-input font-code"
          type="password"
          value={token}
          onChange={(e) => {
            setToken(e.target.value);
            setVerifiedBot(null);
            setChats([]);
          }}
          placeholder={t("channels.telegram.tokenPlaceholder")}
          autoComplete="off"
        />
      </label>

      <div className="flex flex-wrap gap-2 mb-3">
        <button
          type="button"
          className="dw-btn dw-btn-secondary text-sm"
          disabled={verify.isPending || !token.trim()}
          onClick={() => verify.mutate()}
        >
          {verify.isPending ? t("channels.telegram.verifying") : t("channels.telegram.verify")}
        </button>
      </div>

      {verify.isError && (
        <div className="dw-alert-error mb-3 text-sm">{(verify.error as Error).message}</div>
      )}
      {verify.data && !verify.data.ok && verify.data.error && (
        <div className="dw-alert-error mb-3 text-sm">{verify.data.error}</div>
      )}

      {verifiedBot && (
        <div className="dw-alert-success mb-3 text-sm">
          {t("channels.telegram.verified").replace("{username}", verifiedBot.username)}
        </div>
      )}

      {verifiedBot && (
        <>
          <p className="text-secondary text-sm mb-2">{t("channels.telegram.startHint")}</p>
          <div className="flex flex-wrap gap-2 mb-3">
            <button
              type="button"
              className="dw-btn dw-btn-secondary text-sm"
              disabled={fetchChats.isPending}
              onClick={() => fetchChats.mutate()}
            >
              {fetchChats.isPending
                ? t("common.loading")
                : t("channels.telegram.refreshChats")}
            </button>
          </div>
          {fetchChats.isSuccess && chats.length === 0 && (
            <p className="text-secondary text-sm mb-3">{t("channels.telegram.chatsEmpty")}</p>
          )}
          {fetchChats.isError && (
            <div className="dw-alert-error mb-3 text-sm">{(fetchChats.error as Error).message}</div>
          )}
          {chats.length > 0 && (
            <label className="flex flex-col gap-1 text-sm mb-3">
              <span className="text-secondary font-medium">{t("channels.telegram.chatSelect")}</span>
              <select
                className="dw-input font-code"
                value={chatId}
                onChange={(e) => setChatId(e.target.value)}
              >
                <option value="">{t("channels.telegram.chatSelectPlaceholder")}</option>
                {chats.map((c) => (
                  <option key={c.chat_id} value={c.chat_id}>
                    {formatChatLabel(c)}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label className="flex flex-col gap-1 text-sm mb-3">
            <span className="text-secondary font-medium">{t("channels.telegram.chatManual")}</span>
            <input
              className="dw-input font-code"
              value={chatId}
              onChange={(e) => setChatId(e.target.value)}
              placeholder={t("setup.channels.tgChat")}
            />
          </label>
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

      <div className="flex flex-wrap gap-2 mb-2">
        <button
          type="button"
          className="dw-btn dw-btn-primary text-sm"
          disabled={save.isPending || !token.trim() || !verifiedBot}
          onClick={() => save.mutate()}
        >
          {save.isPending ? t("common.saving") : t("common.save")}
        </button>
      </div>

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
        <ExternalNavLink href="https://docs.anycode.dev/guide/telegram" className="dw-link">
          {t("channels.docsLink")}
        </ExternalNavLink>
      </p>
    </div>
  );
}
