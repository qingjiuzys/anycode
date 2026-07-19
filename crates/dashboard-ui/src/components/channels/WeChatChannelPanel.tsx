import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/api/client";
import { ExternalNavLink } from "@/components/ExternalNavLink";
import { useT } from "@/i18n/context";

export function WeChatChannelPanel({
  configured = false,
  platform = "unknown",
  startCommand = "anycode-daemon wechat-bridge",
}: {
  configured?: boolean;
  platform?: string;
  startCommand?: string;
}) {
  const t = useT();
  const qc = useQueryClient();
  const isWindows = platform === "windows";

  const wechatQr = useQuery({
    queryKey: ["wechat-qr"],
    queryFn: () => api.setupWechatQr(),
    enabled: !configured,
    retry: 1,
    staleTime: 0,
  });

  const refreshQr = useMutation({
    mutationFn: () => api.setupWechatQr(),
    onSuccess: (data) => {
      qc.setQueryData(["wechat-qr"], data);
      void qc.removeQueries({ queryKey: ["wechat-poll"] });
    },
  });

  const qrcodeId = wechatQr.data?.qr?.qrcode_id ?? refreshQr.data?.qr?.qrcode_id ?? "";
  const wechatPoll = useQuery({
    queryKey: ["wechat-poll", qrcodeId],
    queryFn: () => api.setupWechatStatus(qrcodeId),
    enabled: !configured && qrcodeId.length > 0,
    refetchInterval: (q) =>
      q.state.data?.result?.status === "confirmed" ? false : 3000,
  });

  const pollStatus = wechatPoll.data?.result?.status;
  const wechatConfirmed = pollStatus === "confirmed";
  const wechatExpired = pollStatus === "expired";

  useEffect(() => {
    if (wechatConfirmed) {
      void qc.invalidateQueries({ queryKey: ["channels-settings"] });
    }
  }, [wechatConfirmed, qc]);

  const copyCommand = async () => {
    try {
      await navigator.clipboard.writeText(startCommand);
    } catch {
      /* ignore */
    }
  };

  const qrLoading = wechatQr.isLoading || refreshQr.isPending;
  const qr = wechatQr.data?.qr ?? refreshQr.data?.qr;
  const qrError =
    wechatQr.isError
      ? (wechatQr.error as Error).message
      : refreshQr.isError
        ? (refreshQr.error as Error).message
        : null;

  return (
    <div className="channel-panel">
      {configured && (
        <div className="dw-alert-success mb-3 text-sm">{t("settings.channels.wechatConfigured")}</div>
      )}

      {isWindows ? (
        <p className="text-secondary text-sm mb-3">{t("setup.channels.wechatWindows")}</p>
      ) : (
        <p className="text-secondary text-sm mb-3">{t("setup.channels.wechatMac")}</p>
      )}

      {!configured && (
        <>
          {qrLoading && <p className="text-secondary text-sm">{t("common.loading")}</p>}

          {qrError && (
            <div className="dw-alert-error mb-3 text-sm">
              {qrError}
              <div className="mt-2">
                <button
                  type="button"
                  className="dw-btn-secondary text-sm"
                  disabled={refreshQr.isPending}
                  onClick={() => refreshQr.mutate()}
                >
                  {refreshQr.isPending ? t("common.loading") : t("channels.wechat.refreshQr")}
                </button>
              </div>
            </div>
          )}

          {!qrLoading && !qrError && !qr && (
            <div className="mb-3">
              <p className="text-secondary text-sm mb-2">{t("channels.wechat.qrUnavailable")}</p>
              <button
                type="button"
                className="dw-btn-secondary text-sm"
                disabled={refreshQr.isPending}
                onClick={() => refreshQr.mutate()}
              >
                {refreshQr.isPending ? t("common.loading") : t("channels.wechat.refreshQr")}
              </button>
            </div>
          )}

          {qr && (
            <div className="setup-wechat-qr mb-3">
              {qr.qr_svg ? (
                <div
                  className="setup-wechat-qr__svg"
                  aria-label="WeChat QR code"
                  dangerouslySetInnerHTML={{ __html: qr.qr_svg }}
                />
              ) : qr.terminal_render ? (
                <pre className="setup-wechat-qr__matrix text-xs whitespace-pre font-code">
                  {qr.terminal_render}
                </pre>
              ) : (
                <p className="font-code text-xs break-all">{qr.content}</p>
              )}
              <p className="text-secondary text-sm mt-2">
                {wechatConfirmed
                  ? t("setup.channels.wechatOk")
                  : wechatExpired
                    ? t("channels.wechat.qrExpired")
                    : t("setup.channels.wechatScan")}
              </p>
              {(wechatExpired || wechatConfirmed) && (
                <button
                  type="button"
                  className="dw-btn-secondary text-sm mt-2"
                  disabled={refreshQr.isPending}
                  onClick={() => refreshQr.mutate()}
                >
                  {refreshQr.isPending ? t("common.loading") : t("channels.wechat.refreshQr")}
                </button>
              )}
            </div>
          )}

          {wechatPoll.isError && (
            <div className="dw-alert-error mb-3 text-sm">{(wechatPoll.error as Error).message}</div>
          )}
        </>
      )}

      {(configured || wechatConfirmed) && (
        <div className="text-sm text-secondary">
          <p className="m-0 mb-1">{t("channels.startBridgeHint")}</p>
          <code className="font-code text-xs">{startCommand}</code>{" "}
          <button type="button" className="dw-link text-sm" onClick={() => void copyCommand()}>
            {t("common.copy")}
          </button>
        </div>
      )}

      <p className="text-xs text-secondary mt-3 m-0">
        <ExternalNavLink href="https://docs.anycode.dev/guide/wechat" className="dw-link">
          {t("channels.docsLink")}
        </ExternalNavLink>
      </p>
    </div>
  );
}
